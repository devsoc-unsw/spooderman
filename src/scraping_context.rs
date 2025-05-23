use crate::{
    config::{FromEnvFile, ScrapingConfig},
    requests::RequestClient,
};

pub struct ScrapingContext {
    pub scraping_config: ScrapingConfig,
    pub request_client: RequestClient,
}

impl ScrapingContext {
    pub fn new() -> anyhow::Result<Self> {
        let config = ScrapingConfig::parse_from_envfile()?;
        let request_client = RequestClient::new()?;
        Ok(ScrapingContext {
            scraping_config: config,
            request_client,
        })
    }
}
