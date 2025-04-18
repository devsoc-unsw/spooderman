use governor::{
    Quota, RateLimiter as GovernorRateLimiter,
    clock::{QuantaClock, QuantaInstant},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
};
use nonzero_ext::nonzero;
use std::{num::NonZeroU32, time::Duration};

// TODO: write script (python?) to find optimal values.
// The higher, the faster.
const REQ_PER_SEC: NonZeroU32 = nonzero!(80u32);
// The lower, the faster.
const MS_BETWEEN_REQ: Duration = Duration::from_millis(10);

type SpecificGovernorRateLimiter =
    GovernorRateLimiter<NotKeyed, InMemoryState, QuantaClock, NoOpMiddleware<QuantaInstant>>;

pub struct RateLimiter {
    req_per_sec: SpecificGovernorRateLimiter,
    ms_between_req: SpecificGovernorRateLimiter,
}

impl RateLimiter {
    pub fn new() -> Self {
        // Limit to X total req/sec on average.
        let req_per_sec = GovernorRateLimiter::direct(Quota::per_second(REQ_PER_SEC));

        // Limit to Y req/ms (i.e. no two requests closer than Y ms).
        let ms_between_req =
            GovernorRateLimiter::direct(Quota::with_period(MS_BETWEEN_REQ).unwrap());

        RateLimiter {
            req_per_sec,
            ms_between_req,
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
        self.req_per_sec.until_ready().await;
        // Y ms have passed since the last time we called this.
        self.ms_between_req.until_ready().await;
    }
}
