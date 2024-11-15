use std::sync::Arc;

use scraper::Selector;
use tokio::sync::Mutex;

use crate::{
    class_scraper::ClassScraper,
    scraper::fetch_url,
    text_manipulators::{extract_text, extract_year, get_html_link_to_page},
    UrlInvalidError,
};

#[derive(Debug)]
pub struct SubjectAreaScraper {
    pub url: Option<String>,
    pub class_scrapers: Vec<Arc<Mutex<ClassScraper>>>,
}

impl SubjectAreaScraper {
    pub async fn scrape(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        match &self.url {
            Some(url) => {
                let html = fetch_url(url)
                    .await
                    .expect("There was something wrong with the URL");
                println!("Scraping Subject Area for: {}", url);
                let career_selector = Selector::parse("td.classSearchMinorHeading").unwrap();
                let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight").unwrap();
                let code_selector = Selector::parse("td.data").unwrap();
                let name_selector = Selector::parse("td.data a").unwrap();
                let link_selector = Selector::parse("td.data a").unwrap();
                let uoc_selector = Selector::parse("td.data:nth-child(3)").unwrap();
                let document = scraper::Html::parse_document(&html);
                for career_elem_ref in document.select(&career_selector) {
                    let career = extract_text(career_elem_ref);
                    if career.is_empty() {continue};
                    for row_node in document.select(&row_selector) {
                        // Extract data from each row
                        let course_code = extract_text(row_node.select(&code_selector).next().unwrap());
                        let course_name = extract_text(row_node.select(&name_selector).nth(1).unwrap());
                        let year_to_scrape = extract_year(url).unwrap(); 
                        let url_to_scrape_further = get_html_link_to_page(
                            year_to_scrape as i32, 
                            row_node
                                .select(&link_selector)
                                .next()
                                .map_or("", |node| node.value().attr("href").unwrap_or("")),
                        );
                        let uoc = extract_text(row_node.select(&uoc_selector).next().unwrap())
                            .parse()
                            .expect("Could not parse UOC!");
                        self.class_scrapers.push(Arc::new(Mutex::new(ClassScraper {
                            course_code,
                            course_name,
                            career: career.trim().to_string(),
                            uoc,
                            url: url_to_scrape_further,
                        })));
                    }
                }

                Ok(())
            }
            None => Err(Box::new(UrlInvalidError)),
        }
    }
}

impl SubjectAreaScraper {
    pub fn new(url: String) -> Self {
        Self {
            url: Some(url),
            class_scrapers: vec![],
        }
    }
}
