use log::warn;
use regex::Regex;
use scraper::ElementRef;

pub fn extract_text(node: ElementRef) -> String {
    node.text().collect::<String>()
}
pub fn extract_year(url: &str) -> Option<u32> {
    let re = Regex::new(r"/(\d{4})/").unwrap();
    if let Some(caps) = re.captures(url) {
        caps.get(1)
            .and_then(|year_match| year_match.as_str().parse::<u32>().ok())
    } else {
        println!("This url caused an error {url}");
        None
    }
}

pub fn mutate_string_to_include_curr_year(curr_base_url: &mut String, year_str: String) -> String {
    let pattern = Regex::new("year").unwrap();
    pattern.replace(&curr_base_url, year_str).to_string()
}

pub fn get_html_link_to_page(year: i32, html_fragment: &str) -> String {
    match std::env::var("TIMETABLE_API_URL") {
        Ok(url) => {
            mutate_string_to_include_curr_year(&mut url.to_string(), year.to_string())
                + html_fragment
        }
        Err(e) => {
            warn!("Timetable URL has NOT been parsed properly from env file and error report: {e}");
            return "".to_string();
        }
    }
}
