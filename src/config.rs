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

/// Regexes that can be used to extract the year and course code from a UNSW
/// timetable url.
pub struct TimetableUrlRegex {
    year_only_regex: Regex,
    course_url_regex: Regex,
}

const YEAR: &str = "YEAR";
const COURSE_CODE: &str = "CODE";

impl TimetableUrlRegex {
    pub fn new() -> anyhow::Result<Self> {
        let year_only_regex = Regex::new(&format!(
            r"^https?://timetable\.unsw\.edu\.au/(?P<{YEAR}>\d{{4}})"
        ))?;
        let course_url_regex = Regex::new(&format!(
            r"^https?://timetable\.unsw\.edu\.au/(?P<{YEAR}>\d{{4}})/(?P<{COURSE_CODE}>[A-Z]{{4}}\d{{4}})\.html$"
        ))?;
        Ok(Self {
            year_only_regex,
            course_url_regex,
        })
    }

    fn extract_year_and_course_code<'a>(
        &self,
        timetable_url: &'a str,
    ) -> anyhow::Result<(Year, &'a str)> {
        let Some(caps) = self.course_url_regex.captures(timetable_url) else {
            return Err(anyhow::anyhow!(
                "failed to apply course url regex to url: {}",
                timetable_url
            ));
        };
        let year: Year = caps
            .name(YEAR)
            .ok_or_else(|| anyhow::anyhow!("missing capture group YEAR"))?
            .as_str()
            .parse()?;
        let code = caps
            .name(COURSE_CODE)
            .ok_or_else(|| anyhow::anyhow!("missing capture group COURSE_CODE"))?
            .as_str();
        Ok((year, code))
    }

    pub fn extract_year(&self, timetable_url: &str) -> anyhow::Result<Year> {
        let Some(caps) = self.year_only_regex.captures(timetable_url) else {
            return Err(anyhow::anyhow!(
                "failed to apply year only regex to url: {}",
                timetable_url
            ));
        };
        let year: Year = caps
            .name(YEAR)
            .ok_or_else(|| anyhow::anyhow!("missing capture group YEAR"))?
            .as_str()
            .parse()?;
        Ok(year)
    }

    pub fn extract_course_code<'a>(&self, timetable_url: &'a str) -> anyhow::Result<&'a str> {
        let (_year, code) = self.extract_year_and_course_code(timetable_url)?;
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timetable_url_regex() {
        let timetable_url_regex = TimetableUrlRegex::new().unwrap();
        assert_eq!(
            2024,
            timetable_url_regex
                .extract_year("https://timetable.unsw.edu.au/2024")
                .unwrap()
        );
        assert_eq!(
            2024,
            timetable_url_regex
                .extract_year("https://timetable.unsw.edu.au/2024/COMP1511.html")
                .unwrap()
        );
        assert_eq!(
            "COMP1511",
            timetable_url_regex
                .extract_course_code("https://timetable.unsw.edu.au/2024/COMP1511.html")
                .unwrap()
        );
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
