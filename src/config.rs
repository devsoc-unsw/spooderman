use anyhow::Context;
use serde::{Deserialize, de::DeserializeOwned};

/// The env config env vars needed for scraping.
#[derive(Debug, Deserialize)]
pub struct ScrapingConfig {
    pub timetable_api_url: String,
}

/// The env config env vars needed for uploading scraped data.
#[derive(Debug, Deserialize)]
pub struct UploadingConfig {
    pub hasuragres_url: String,
    pub hasuragres_api_key: String,
}

// Extension trait.
pub trait FromEnvFile: DeserializeOwned {
    fn parse_from_envfile() -> anyhow::Result<Self> {
        dotenv::dotenv().context("failed to load `.env`")?;
        let config =
            envy::from_env::<Self>().context("env file exists, but failed to parse config")?;
        Ok(config)
    }
}

impl<T: DeserializeOwned> FromEnvFile for T {}
