use governor::{
    Quota, RateLimiter as GovernorRateLimiter,
    clock::{QuantaClock, QuantaInstant},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use nonzero_ext::nonzero;
use std::{
    fmt::{self},
    num::NonZeroU32,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

use crate::Request;

// The higher, the faster.
const DEFAULT_REQ_PER_SEC: NonZeroU32 = nonzero!(150u32);

// The lower, the faster.
const DEFAULT_MS_BETWEEN_REQ: Duration = Duration::from_millis(2);

// The higher, the faster the backoff.
const LINEAR_REQUEST_RATE_BACKOFF: NonZeroU32 = nonzero!(30u32);

#[derive(Debug, Clone, Copy)]
struct RequestRate {
    req_per_sec: NonZeroU32,
    ms_between_req: Duration,
    most_recently_updated: Instant,
}

impl RequestRate {
    fn new(
        req_per_sec: NonZeroU32,
        ms_between_req: Duration,
        most_recently_updated: Instant,
    ) -> Self {
        Self {
            req_per_sec,
            ms_between_req,
            most_recently_updated,
        }
    }
}

impl fmt::Display for RequestRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} requests per second", self.req_per_sec)?;
        Ok(())
    }
}

type SpecificGovernorRateLimiter =
    GovernorRateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

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

    fn update_request_rate(&mut self, new_request_rate: RequestRate) {
        log::info!(
            "updating the request rate from '{}' to '{}'",
            self.request_rate,
            new_request_rate
        );
        *self = Self::new(new_request_rate);
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

pub struct RateLimiter {
    fixed_rate_limiter: RwLock<FixedRateLimiter>,
}

fn sub_nonzero(a: NonZeroU32, b: NonZeroU32) -> Option<NonZeroU32> {
    a.get().checked_sub(b.get()).and_then(NonZeroU32::new)
}

impl RateLimiter {
    pub fn new() -> Self {
        let request_rate =
            RequestRate::new(DEFAULT_REQ_PER_SEC, DEFAULT_MS_BETWEEN_REQ, Instant::now());
        let fixed_rate_limiter = RwLock::new(FixedRateLimiter::new(request_rate));
        Self { fixed_rate_limiter }
    }

    pub async fn wait_until_ready(&self) {
        let fixed_rate_limiter = self.fixed_rate_limiter.read().await;
        fixed_rate_limiter.wait_until_ready().await;
    }

    pub async fn lower_request_rate(&self, failed_request: Request) -> anyhow::Result<()> {
        let (most_recently_updated, old_request_rate) = {
            let fixed_rate_limiter = self.fixed_rate_limiter.read().await;
            (
                fixed_rate_limiter.request_rate.most_recently_updated,
                fixed_rate_limiter.request_rate,
            )
        };

        // If the failed request (A) was made before the most recent request rate
        // reduction (which was made due to failed request B), then A failing
        // shouldn't lower the request rate even further, since we will have
        // hundreds of failed requests like A, so reducing for all of them would cause
        // our request rate to go to 0 immediately. Only the next failed request C
        // after the latest request rate update should cause another request rate
        // update, since the current request rate clearly isn't low enough at the point
        // of C either.
        if failed_request.sent_time <= most_recently_updated {
            return Ok(());
        }

        {
            let mut fixed_rate_limiter = self.fixed_rate_limiter.write().await;

            let Some(new_req_per_sec) =
                sub_nonzero(old_request_rate.req_per_sec, LINEAR_REQUEST_RATE_BACKOFF)
            else {
                let err_msg = "the request rate has been lowered to 0 req/sec so we can't lower it any further";
                log::error!("{}", err_msg);
                return Err(anyhow::anyhow!(err_msg));
            };
            let new_ms_beteen_reqs = old_request_rate.ms_between_req;
            let new_request_rate =
                RequestRate::new(new_req_per_sec, new_ms_beteen_reqs, Instant::now());

            // Before switching to the new request rate, we should back off
            // completely for a couple of seconds: kill all in-flight requests,
            // and stop sending new requests for a couple of seconds.
            // TODO

            fixed_rate_limiter.update_request_rate(new_request_rate);
        }

        // log::info!("pausing requests for 1 second");
        // tokio::time::sleep(Duration::from_secs(1)).await;

        Ok(())
    }
}
