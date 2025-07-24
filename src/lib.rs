mod config;
mod course_scraper;
mod hasuragres_b_insert;
mod ratelimit;
mod requests;
mod school_area_scraper;
mod scraping_context;
mod subject_area_scraper;
mod text_manipulators;
mod url_invalid_error;
mod utils;

pub use config::{ScrapingEnv, UploadingConfig};
pub use course_scraper::{Class, Course, PartialCourse, Time};
pub use hasuragres_b_insert::{ReadFromFile, ReadFromMemory, send_batch_data};
pub use requests::RequestClient;
pub use school_area_scraper::SchoolArea;
pub use scraping_context::ScrapingContext;
pub use url_invalid_error::UrlInvalidError;
pub use utils::sort_by_key_ref;

// NOTE: i32 because that is what DateTime<Utc> uses for years.
pub type Year = i32;
