mod scraper;
mod url_invalid_error;

mod class_scraper;
mod school_area_scraper;
mod subject_area_scraper;
mod text_manipulators;

pub use scraper::fetch_url;
pub use scraper::Scraper;
pub use url_invalid_error::UrlInvalidError;
// pub use subject_area_scraper::SubjectAreaScraper;
pub use class_scraper::{ClassScraper, Course};
pub use school_area_scraper::SchoolAreaScraper;
pub use subject_area_scraper::SubjectAreaScraper;
