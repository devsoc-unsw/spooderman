use log::warn;
use regex::Regex;
use scraper::ElementRef;

use chrono::{Datelike, Utc};
pub fn extract_text(node: ElementRef) -> String {
    node.text().collect::<String>()
}

pub fn mutate_string_to_include_curr_year(curr_base_url: &mut String) -> String {
    let pattern = Regex::new("year").unwrap();
    pattern
        .replace(&curr_base_url, Utc::now().year().to_string())
        .to_string()
}

pub fn get_html_link_to_page(html_fragment: &str) -> String {
    match std::env::var("TIMETABLE_API_URL") {
        Ok(url) => {
            mutate_string_to_include_curr_year(&mut url.to_string()) + html_fragment
        }
        Err(e) => {
            warn!("Timetable URL has NOT been parsed properly from env file and error report: {e}");
            return "".to_string();
        }
    }
}
