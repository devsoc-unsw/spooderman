use scraper::ElementRef;

use crate::{ScrapingContext, Year};

pub fn extract_text(node: ElementRef) -> String {
    node.text().collect::<String>()
}

pub fn get_html_link_to_page(year: Year, html_fragment: &str, ctx: &ScrapingContext) -> String {
    ctx.scraping_config.get_timetable_api_url_for_year(year) + html_fragment
}
