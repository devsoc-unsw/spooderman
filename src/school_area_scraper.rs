use crate::{
    scraper::fetch_url,
    subject_area_scraper::SubjectAreaScraper,
    text_manipulators::{extract_text, extract_year, get_html_link_to_page},
};
use scraper::Selector;
use std::{error::Error, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct SchoolAreaPage {
    pub course_code: String,
    pub course_name: String,
    pub school: String,
    pub subject_area_scraper: Arc<Mutex<SubjectAreaScraper>>,
}

#[derive(Debug)]
pub struct SchoolAreaScraper {
    pub url: Option<String>,
    pub pages: Vec<SchoolAreaPage>,
}

#[derive(Debug)]
pub struct ScrapeError {
    details: String,
}

impl std::fmt::Display for ScrapeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ScrapeError: {}", self.details)
    }
}
impl Error for ScrapeError {}

impl SchoolAreaScraper {
    pub async fn scrape(&mut self) -> Result<(), Box<ScrapeError>> {
        match &self.url {
            Some(url) => {
                println!("School Area for: {}", url);
                let html = fetch_url(url)
                    .await
                    .expect("There has been something wrong with the URL.");
                let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight").unwrap();
                let code_selector = Selector::parse("td.data").unwrap();

                let name_selector = Selector::parse("td.data a").unwrap();
                let link_selector = Selector::parse("td.data a").unwrap();

                let school_selector = Selector::parse("td.data:nth-child(3)").unwrap();
                let document = scraper::Html::parse_document(&html);
                for row_node in document.select(&row_selector) {
                    // Extract data from each row
                    let course_code = extract_text(row_node.select(&code_selector).next().unwrap());
                    let course_name = extract_text(row_node.select(&name_selector).next().unwrap());
                    let url_to_scrape_further = get_html_link_to_page(
                        extract_year(url).unwrap() as i32,
                        row_node
                            .select(&link_selector)
                            .next()
                            .map_or("", |node| node.value().attr("href").unwrap_or("")),
                    );

                    let school = extract_text(row_node.select(&school_selector).next().unwrap());
                    let page = SchoolAreaPage {
                        course_code,
                        course_name,
                        school,
                        subject_area_scraper: Arc::new(Mutex::new(SubjectAreaScraper::new(url_to_scrape_further))),
                    };

                    self.pages.push(page);
                }

                Ok(())
            }
            None => Err(Box::new(ScrapeError {
                details: "There was something wrong with scraping the class".to_string(),
            })),
        }
    }
}

impl SchoolAreaScraper {
    pub fn new(url: String) -> Self {
        SchoolAreaScraper {
            url: Some(url),
            pages: Vec::new(),
        }
    }
}
