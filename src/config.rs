use anyhow::Context;
use regex::Regex;
use serde::{Deserialize, de::DeserializeOwned};

use crate::Year;

/// The env config env vars needed for scraping.
#[derive(Debug, Deserialize)]
pub struct ScrapingEnv {
    timetable_api_url: String,
}

pub struct ScrapingConfig {
    timetable_api_url: String,
}

impl ScrapingConfig {
    pub fn new() -> anyhow::Result<Self> {
        let scraping_env = ScrapingEnv::load_from_env()?;
        Ok(Self {
            timetable_api_url: scraping_env.timetable_api_url,
        })
    }

    pub fn get_timetable_api_url_for_year(&self, year: Year) -> String {
        self.timetable_api_url.replace("year", &year.to_string())
    }
}

pub struct TimetableUrlYearExtractor {
    // Regex that can be used to extract the year from a a UNSW timetable url.
    year_extraction_regex: Regex,
}

impl TimetableUrlYearExtractor {
    pub fn new() -> anyhow::Result<Self> {
        let year_extraction_regex = Regex::new(r"/(\d{4})/")?;
        Ok(Self {
            year_extraction_regex,
        })
    }

    pub fn extract_year(&self, timetable_url: &str) -> anyhow::Result<Year> {
        let Some(caps) = self.year_extraction_regex.captures(timetable_url) else {
            return Err(anyhow::anyhow!(
                "couldn't find year in provided url: {}",
                timetable_url
            ));
        };
        let Some(match_) = caps.get(1) else {
            return Err(anyhow::anyhow!(
                "couldn't find year in provided url: {}",
                timetable_url
            ));
        };
        let year = match_.as_str().parse::<Year>()?;
        Ok(year)
    }
}

/// The env config env vars needed for uploading scraped data.
#[derive(Debug, Deserialize)]
pub struct UploadingConfig {
    pub hasuragres_url: String,
    pub hasuragres_api_key: String,
}

// Extension trait.
pub trait LoadFromEnv: DeserializeOwned {
    fn load_from_env() -> anyhow::Result<Self> {
        // Don't throw an error if .env file doesn't exist.
        let _ = dotenv::dotenv();
        let config =
            envy::from_env::<Self>().context("failed to load env variables into config struct")?;
        Ok(config)
    }
}

impl<T: DeserializeOwned> LoadFromEnv for T {}
