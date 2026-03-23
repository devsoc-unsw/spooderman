use governor::{
    Quota, RateLimiter as GovernorRateLimiter,
    clock::{QuantaClock, QuantaInstant},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use nonzero_ext::nonzero;
use num_traits::ToPrimitive;
use std::{
    cmp::max,
    fmt::{self},
    num::NonZeroU32,
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{RwLock, watch},
    time::Instant,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::Request;

// The higher, the faster.
const DEFAULT_REQ_PER_SEC: NonZeroU32 = nonzero!(100u32);

// The lower, the faster.
const DEFAULT_MS_BETWEEN_REQ: Duration = Duration::from_millis(2);

// The closer to 1, the slower the request rate goes to 0.
const EXPONENTIAL_REQUEST_RATE_BACKOFF: f64 = 2.0 / 3.0;

// The lower, the faster we restart after request rate change.
const PAUSE_AFTER_REQ_RATE_CHANGE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy)]
pub struct RequestRate {
    req_per_sec: NonZeroU32,
    ms_between_req: Duration,
    uuid: Uuid,
}

impl PartialEq for RequestRate {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

impl RequestRate {
    fn new(req_per_sec: NonZeroU32, ms_between_req: Duration) -> Self {
        let uuid = Uuid::new_v4();
        Self {
            req_per_sec,
            ms_between_req,
            uuid,
        }
    }
}

impl fmt::Display for RequestRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: This isn't a unique repr of this request rate, but not printing
        // UUID for reduced verbosity.
        write!(f, "{} req/s", self.req_per_sec)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
enum Gate {
    Active,
    PausedUntil(Instant),
}

type SpecificGovernorRateLimiter =
    GovernorRateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

#[derive(Debug)]
struct FixedRateLimiter {
    request_rate: RequestRate,
    req_per_sec_rate_limiter: SpecificGovernorRateLimiter,
    ms_between_req_rate_limiter: SpecificGovernorRateLimiter,
}

impl FixedRateLimiter {
    fn new(request_rate: RequestRate) -> Self {
        // Limit to X total req/sec on average.
        let req_per_sec_rate_limiter =
            GovernorRateLimiter::direct(Quota::per_second(request_rate.req_per_sec));

        // Limit to Y req/ms (i.e. no two requests closer than Y ms).
        let ms_between_req_rate_limiter = GovernorRateLimiter::direct(
            Quota::with_period(request_rate.ms_between_req)
                .expect("millis between requests should be larger than 0"),
        );

        Self {
            request_rate,
            req_per_sec_rate_limiter,
            ms_between_req_rate_limiter,
        }
    }

    pub async fn wait_until_ready(&self) {
        // The order in which we await the rate limiters matters:

        // If our wait time between requests is very small (relative to the
        // requests per sec limiter), then we might queue up many callers that
        // got past the Y ms period, which can then all concurrently cross the
        // requests per second boundary once that allows more "flow".

        // If our requests per second is very high (relative to the wait time
        // between requests), then many callers might clear the requests per
        // second hurdle quickly, but would get stuck at the time between req
        // check, which will be strict and only allow 1 caller to pass every Y ms.

        // We won't call this more than X times per sec.
        self.req_per_sec_rate_limiter.until_ready().await;
        // Y ms have passed since the last time we called this.
        self.ms_between_req_rate_limiter.until_ready().await;
    }
}

impl Drop for FixedRateLimiter {
    fn drop(&mut self) {
        log::info!(
            "rate limiter with request rate {} is no longer referenced",
            self.request_rate
        )
    }
}

#[derive(Debug)]
struct RateLimiterGeneration {
    rate_limiter: FixedRateLimiter,
    cancel_token: CancellationToken,
}

impl RateLimiterGeneration {
    fn new(fixed_rate_limiter: FixedRateLimiter) -> Self {
        Self {
            rate_limiter: fixed_rate_limiter,
            cancel_token: CancellationToken::new(),
        }
    }
}

#[derive(Debug)]
pub struct RateLimiter {
    // Hot-swappable rate limiter, wrapped in an Arc to avoid holding lock during waits.
    rate_limiter_generation: RwLock<Arc<RateLimiterGeneration>>,
    // Gate used to allow pausing all requests for a period of time.
    gate_tx: watch::Sender<Gate>,
    gate_rx: watch::Receiver<Gate>,
}

impl RateLimiter {
    pub fn new() -> Self {
        let request_rate = RequestRate::new(DEFAULT_REQ_PER_SEC, DEFAULT_MS_BETWEEN_REQ);
        let fixed_rate_limiter = FixedRateLimiter::new(request_rate);
        let rate_limiter_generation =
            RwLock::new(Arc::new(RateLimiterGeneration::new(fixed_rate_limiter)));
        let (gate_tx, gate_rx) = watch::channel(Gate::Active);

        Self {
            rate_limiter_generation,
            gate_tx,
            gate_rx,
        }
    }

    async fn wait_until_active(&self) {
        loop {
            let curr_gate = *self.gate_rx.borrow();
            match curr_gate {
                Gate::Active => return,
                Gate::PausedUntil(_deadline) => {
                    let mut rx = self.gate_rx.clone();
                    rx.changed()
                        .await
                        .expect("sender can't get dropped, it's part of RateLimiter struct");
                }
            }
        }
    }

    async fn pause_for(&self, duration: Duration) {
        log::info!(
            "Pausing new requests for {}",
            humantime::Duration::from(duration)
        );

        let new_deadline = Instant::now() + duration;

        // Start or extend the pause atomically.
        let _ = self.gate_tx.send_modify(|gate| {
            *gate = match *gate {
                Gate::Active => Gate::PausedUntil(new_deadline),
                Gate::PausedUntil(curr_deadline) => {
                    let max_deadline = max(curr_deadline, new_deadline);
                    Gate::PausedUntil(max_deadline)
                }
            };
        });

        // Schedule unpause at deadline.
        let tx = self.gate_tx.clone();
        tokio::task::spawn(async move {
            tokio::time::sleep_until(new_deadline).await;
            let _ = tx.send_modify(|gate| {
                *gate = match *gate {
                    Gate::Active => Gate::Active,
                    Gate::PausedUntil(curr_deadline) => {
                        if curr_deadline > new_deadline {
                            // After setting this new deadline, we set a later deadline,
                            // so that later deadline should be respected.
                            Gate::PausedUntil(curr_deadline)
                        } else if curr_deadline < new_deadline {
                            // After setting this new deadline, we set another earlier deadline.
                            // However, in that case, the wake-task for that earlier deadline
                            // should have set the gate to `Active`, so this should never be reached.
                            unreachable!("we should never see an earlier deadline on wakeup, as that implies we would have woken up earlier and resolved the earlier deadline");
                        } else {
                            // This is the deadline we set. If the same deadline was set twice,
                            // the second modify would just encounter an `Active`, which it leaves
                            // untouched.
                            log::info!("Resuming requests");
                            Gate::Active
                        }
                    }
                };
            });
        });
    }
}

pub enum PermitResult {
    /// The thread has finished waiting and has now been granted passage, and
    /// receives the request rate that was used while waiting.
    Granted { request_rate_used: RequestRate },
    /// The operation the thread is waiting to be allowed to perform was cancelled.
    Cancelled,
}

impl RateLimiter {
    pub async fn wait_until_ready(&self) -> PermitResult {
        // Don't hold the read lock while waiting on rate limiter, so writer can quickly get write lock.
        let generation = { Arc::clone(&*self.rate_limiter_generation.read().await) };

        let wait_until_active_and_allowed = async {
            self.wait_until_active().await;
            generation.rate_limiter.wait_until_ready().await
        };

        tokio::select! {
            _ = wait_until_active_and_allowed => {
                PermitResult::Granted { request_rate_used: generation.rate_limiter.request_rate }
            },
            _ = generation.cancel_token.cancelled() => {
                PermitResult::Cancelled
            }
        }
    }

    pub async fn lower_request_rate<'a, 'b>(
        &self,
        failed_request: Request<'a, 'b>,
    ) -> anyhow::Result<()> {
        {
            // Hold the write lock until update is complete: important to ensure there is
            // only one failed request that wins/comes first.
            let mut generation = self.rate_limiter_generation.write().await;

            let curr_request_rate = &generation.rate_limiter.request_rate;

            // If the failed request (A) was made before the most recent request rate
            // reduction (which was made due to failed request B), then A failing
            // shouldn't lower the request rate even further, since we will have
            // hundreds of failed requests like A, so reducing for all of them would cause
            // our request rate to go to 0 immediately. Only the next failed request C
            // after the latest request rate update should cause another request rate
            // update, since the current request rate clearly isn't low enough at the point
            // of C either.
            // The few in-flight requests that go through even after setting up the
            // new rate limiter do so using the old rate limiter (which is possible
            // because it is shared as an Arc that readers clone, allowing them to drop
            // the lock before proceding). Therefore, we can filter out any request that
            // was made using some old request rate (uniquely identified by uuid).
            if &failed_request.request_rate_used != curr_request_rate {
                log::info!(
                    "request to {} failed with old request rate {}, so ignore failure",
                    failed_request,
                    failed_request.request_rate_used,
                );
                return Ok(());
            }

            let old_request_rate = curr_request_rate;
            let new_req_per_sec_maybe_zero = f64::floor(
                f64::from(old_request_rate.req_per_sec.get()) * EXPONENTIAL_REQUEST_RATE_BACKOFF,
            )
            .to_u32()
            .expect("request rate will never be negative or too large to represent in a u32");
            let Ok(new_req_per_sec) = NonZeroU32::try_from(new_req_per_sec_maybe_zero) else {
                let err_msg = "request rate is 0 req/sec, can't be lowered further";
                log::error!("{}", err_msg);
                return Err(anyhow::anyhow!(err_msg));
            };

            let new_ms_beteen_reqs = old_request_rate.ms_between_req;
            let new_request_rate = RequestRate::new(new_req_per_sec, new_ms_beteen_reqs);

            log::info!(
                "updating request rate from '{}' to '{}' due to failed request to {}",
                old_request_rate,
                new_request_rate,
                failed_request
            );

            // Before switching to the new request rate, we should back off
            // completely for a while.
            self.pause_for(PAUSE_AFTER_REQ_RATE_CHANGE).await;

            // Kill all spawned requests that are currently waiting on the old
            // rate limiter.
            // It is important for the tasks to be cancelled after the rate limiter
            // is paused, otherwise tasks will race to the next loop iteration and
            // might read the old rate limiter active status before it is set to paused.
            generation.cancel_token.cancel();

            // All in-flight requests (those that have made it past our old rate limiter)
            // will be allowed to complete, and they will most likely also be rate-limited
            // by UNSW servers. We allow these requests to progress in order to encapsulate
            // the cancellation logic in this struct rather than requiring the caller to
            // setup the cancellation logic themselves.
            // Once all requests to the old rate limiter have completed, it will be dropped.

            let new_rate_limiter = FixedRateLimiter::new(new_request_rate);
            let new_generation = Arc::new(RateLimiterGeneration::new(new_rate_limiter));
            *generation = new_generation;
        }

        Ok(())
    }
}
