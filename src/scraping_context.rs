use crate::{
    config::{ScrapingConfig, TimetableUrlRegex},
    requests::RequestClient,
};

#[derive(Debug)]
pub struct ScrapingContext {
    pub scraping_config: ScrapingConfig,
    pub timetable_url_regex: TimetableUrlRegex,
    pub request_client: RequestClient,
}

impl ScrapingContext {
    pub fn new() -> anyhow::Result<Self> {
        let scraping_config = ScrapingConfig::new()?;
        let timetable_url_regex = TimetableUrlRegex::new()?;
        let request_client = RequestClient::new()?;
        Ok(ScrapingContext {
            scraping_config,
            timetable_url_regex,
            request_client,
        })
    }
}
