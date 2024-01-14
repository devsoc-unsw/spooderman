mod scraper;
mod url_invalid_error;

mod class_scraper;
mod subject_area_scraper;
mod text_manipulators;

pub use class_scraper::ClassScraper;
pub use scraper::Scraper;
pub use subject_area_scraper::SubjectAreaScraper;
pub use url_invalid_error::UrlInvalidError;
