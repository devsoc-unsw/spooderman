mod scraper;
mod url_invalid_error;

mod school_area_scraper;
mod class_scraper;
mod subject_area_scraper;
mod text_manipulators;

pub use scraper::Scraper;
pub use scraper::fetch_url;
pub use url_invalid_error::UrlInvalidError;
// pub use subject_area_scraper::SubjectAreaScraper;
pub use school_area_scraper::SchoolAreaScraper;
pub use subject_area_scraper::SubjectAreaScraper;
pub use class_scraper::ClassScraper;