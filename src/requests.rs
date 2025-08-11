use std::time::Duration;

use derive_new::new;
use reqwest::{Client, ClientBuilder, Response};

use crate::ratelimit::{RateLimiter, RequestRate};

pub struct RequestClient {
    client: Client,
    rate_limiter: RateLimiter,
}

#[derive(Debug, new)]
pub struct Request<'a> {
    /// The timestamp at which the client sent this request, regardless of when
    /// the server responded and whether the server responded at all.
    // pub sent_time: Instant,
    pub url: &'a str,
    pub request_rate_used: RequestRate,
}

impl RequestClient {
    pub fn new() -> anyhow::Result<Self> {
        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;
        let rate_limiter = RateLimiter::new();
        Ok(Self {
            client,
            rate_limiter,
        })
    }

    pub async fn fetch_url_response(&self, url: &str) -> anyhow::Result<Response> {
        loop {
            // Wait (non-blocking) until we're allowed to make a request according
            // to our self-imposed rate-limiting policy.
            let request_rate_used = self.rate_limiter.wait_until_ready().await;

            log::info!("allowed access to start making request to {} now", url);
            let request = Request::new(url, request_rate_used);
            let failed_request = {
                match tokio::time::timeout(Duration::from_secs(3), self.client.get(url).send())
                    .await
                {
                    Ok(Ok(response)) => return Ok(response),
                    Ok(Err(e)) => {
                        log::warn!(
                            "get-request to url {} failed ({}), maybe reduce request rate",
                            url,
                            e
                        );
                        request
                    }
                    Err(_) => {
                        log::warn!(
                            "get-request to url {} timed out (maybe UNSW servers are rate-limiting us by responding very slowly instead of returning an error), maybe reduce request rate",
                            url,
                        );
                        request
                    }
                }
            };

            // If we got rate-limited by UNSW servers, we are probably making too
            // many requests, so we should send requests at a lower rate (i.e.
            // rate-limit ourselves more).
            self.rate_limiter.lower_request_rate(failed_request).await?;
        }
    }

    pub async fn fetch_url_body(&self, url: &str) -> anyhow::Result<String> {
        let response = self.fetch_url_response(url).await?;
        let body = response.text().await?;
        Ok(body)
    }
}
