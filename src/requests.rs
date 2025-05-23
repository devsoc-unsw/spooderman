use reqwest::{Client, ClientBuilder, Response};

use crate::ratelimit::RateLimiter;

pub struct RequestClient {
    client: Client,
    rate_limiter: RateLimiter,
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
        // Wait (non-blocking) until we're allowed to make a request according
        // to our self-imposed rate-limiting policy.
        self.rate_limiter.wait_until_ready().await;

        let response = self.client.get(url).send().await?;
        Ok(response)
    }

    pub async fn fetch_url_body(&self, url: &str) -> anyhow::Result<String> {
        let response = self.fetch_url_response(url).await?;
        let body = response.text().await?;
        Ok(body)
    }
}
