mod url_invalid_error;

mod course_scraper;
mod hasuragres_b_insert;
mod ratelimit;
mod requests;
mod school_area_scraper;
mod subject_area_scraper;
mod text_manipulators;

pub use course_scraper::{Class, Course, PartialCourse, Time};
pub use hasuragres_b_insert::{ReadFromFile, ReadFromMemory, send_batch_data};
pub use requests::RequestClient;
pub use school_area_scraper::SchoolArea;
pub use text_manipulators::mutate_string_to_include_curr_year;
pub use url_invalid_error::UrlInvalidError;
