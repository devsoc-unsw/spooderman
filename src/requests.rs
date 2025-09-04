use std::{fmt, time::Duration};

use derive_new::new;
use reqwest::{Client, ClientBuilder, StatusCode};

use crate::{
    ScrapingContext,
    ratelimit::{PermitResult, RateLimiter, RequestRate},
};

const GET_REQUEST_TIMEOUT: Duration = Duration::from_secs(3);
const RESPONSE_BODY_TIMEOUT: Duration = Duration::from_secs(3);

pub struct RequestClient {
    client: Client,
    rate_limiter: RateLimiter,
}

#[derive(new)]
pub struct Request<'a, 'b> {
    url: &'a str,
    pub request_rate_used: RequestRate,
    /// Most of the requests sent using the RequestClient are to course pages,
    /// so extract the course code for shorter logs.
    maybe_course_code: &'b Option<&'a str>,
}

impl<'a, 'b> fmt::Display for Request<'a, 'b> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.maybe_course_code.unwrap_or(self.url))?;
        Ok(())
    }
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

    async fn fetch_url_response_and_body(
        &self,
        url: &str,
        ctx: &ScrapingContext,
    ) -> anyhow::Result<(StatusCode, String)> {
        let maybe_course_code = ctx.timetable_url_regex.extract_course_code(url).ok();
        loop {
            // Wait (non-blocking) until we're allowed to make a request according
            // to our self-imposed rate-limiting policy.
            match self.rate_limiter.wait_until_ready().await {
                PermitResult::Granted { request_rate_used } => {
                    let request = Request::new(url, request_rate_used, &maybe_course_code);
                    let failed_request = {
                        match tokio::time::timeout(GET_REQUEST_TIMEOUT, self.client.get(url).send())
                            .await
                        {
                            Ok(Ok(response)) => {
                                // The server might still rate-limit us by sending the response body very slowly.
                                let status = response.status().clone();
                                match tokio::time::timeout(RESPONSE_BODY_TIMEOUT, response.text())
                                    .await
                                {
                                    Ok(Ok(body)) => return Ok((status, body)),
                                    Ok(Err(e)) => {
                                        log::warn!(
                                            "fetching body for {} failed ({}) using {}, maybe reduce request rate",
                                            request,
                                            e,
                                            request_rate_used
                                        );
                                        request
                                    }
                                    Err(_) => {
                                        // Maybe UNSW servers are rate-limiting us by responding very slowly instead of returning an error.
                                        log::warn!(
                                            "fetching body for {} timed out using {}, maybe reduce request rate",
                                            request,
                                            request_rate_used
                                        );
                                        request
                                    }
                                }
                            }
                            Ok(Err(e)) => {
                                log::warn!(
                                    "get {} failed ({}) using {}, maybe reduce request rate",
                                    request,
                                    e,
                                    request_rate_used
                                );
                                request
                            }
                            Err(_) => {
                                log::warn!(
                                    "get {} timed out using {}, maybe reduce request rate",
                                    request,
                                    request_rate_used
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
                PermitResult::Cancelled => {
                    // If this rate-limit-wait was cancelled, try again.
                    continue;
                }
            }
        }
    }

    pub async fn fetch_url_status(
        &self,
        url: &str,
        ctx: &ScrapingContext,
    ) -> anyhow::Result<StatusCode> {
        let (status, _body) = self.fetch_url_response_and_body(url, ctx).await?;
        Ok(status)
    }

    pub async fn fetch_url_body(&self, url: &str, ctx: &ScrapingContext) -> anyhow::Result<String> {
        let (_response, body) = self.fetch_url_response_and_body(url, ctx).await?;
        Ok(body)
    }
}
