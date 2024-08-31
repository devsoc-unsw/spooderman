mod scraper;
mod url_invalid_error;

mod class_scraper;
mod hasuragres_b_insert;
mod school_area_scraper;
mod subject_area_scraper;
mod text_manipulators;

pub use hasuragres_b_insert::{send_batch_data, ReadFromFile, ReadFromMemory};
pub use scraper::fetch_url;
pub use scraper::Scraper;
pub use text_manipulators::mutate_string_to_include_curr_year;
pub use url_invalid_error::UrlInvalidError;
// pub use subject_area_scraper::SubjectAreaScraper;
pub use class_scraper::{Class, ClassScraper, Course, Time};
pub use school_area_scraper::SchoolAreaScraper;
pub use subject_area_scraper::SubjectAreaScraper;
