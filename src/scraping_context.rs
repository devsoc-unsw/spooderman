use crate::{
    config::{ScrapingConfig, TimetableUrlYearExtractor},
    requests::RequestClient,
};

pub struct ScrapingContext {
    pub scraping_config: ScrapingConfig,
    pub timetable_url_year_extractor: TimetableUrlYearExtractor,
    pub request_client: RequestClient,
}

impl ScrapingContext {
    pub fn new() -> anyhow::Result<Self> {
        let scraping_config = ScrapingConfig::new()?;
        let timetable_url_year_extractor = TimetableUrlYearExtractor::new()?;
        let request_client = RequestClient::new()?;
        Ok(ScrapingContext {
            scraping_config,
            timetable_url_year_extractor,
            request_client,
        })
    }
}
