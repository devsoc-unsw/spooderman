use scraper::ElementRef;

pub fn extract_text(node: ElementRef) -> String {
    node.text().collect::<String>()
}

pub fn get_html_link_to_page(html_fragment: &str) -> String {
    "https://timetable.unsw.edu.au/2024/".to_string() + html_fragment
}
