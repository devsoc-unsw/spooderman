use std::borrow::Cow;

use regex::Regex;
use scraper::ElementRef;

use crate::ScrapingContext;

pub fn extract_text(node: ElementRef) -> String {
    node.text().collect::<String>()
}
pub fn extract_year(url: &str) -> Option<u32> {
    let re = Regex::new(r"/(\d{4})/").unwrap();
    if let Some(caps) = re.captures(url) {
        caps.get(1)
            .and_then(|year_match| year_match.as_str().parse::<u32>().ok())
    } else {
        log::error!("This url caused an error {url}");
        None
    }
}

pub fn mutate_string_to_include_curr_year(curr_base_url: &str, year: i32) -> Cow<str> {
    let pattern = Regex::new("year").unwrap();
    let year_str = year.to_string();
    pattern.replace(curr_base_url, year_str)
}

pub fn get_html_link_to_page(year: i32, html_fragment: &str, ctx: &ScrapingContext) -> String {
    mutate_string_to_include_curr_year(&ctx.scraping_config.timetable_api_url, year).to_string()
        + html_fragment
}
