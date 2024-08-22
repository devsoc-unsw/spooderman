use log::info;
use scraper::Selector;

use crate::{
    class_scraper::Course, scraper::fetch_url, subject_area_scraper::{self, SubjectAreaScraper}, text_manipulators::{extract_text, get_html_link_to_page}, Scraper, UrlInvalidError
};


#[derive(Debug)]
pub struct SchoolAreaPage {
    pub subject_area_course_code: String,
    pub subject_area_course_name: String,
    pub school: String,
    pub subject_area_scraper: SubjectAreaScraper,
}

#[derive(Debug)]
pub struct SchoolAreaScraper {
    pub url: Option<String>,
    pub pages: Vec<SchoolAreaPage>,
}

impl Scraper for SchoolAreaScraper {
    async fn scrape(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.url {
            Some(url) => {
                let html = fetch_url(url).await?;
                let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight").unwrap();
                let code_selector = Selector::parse("td.data").unwrap();
                let name_selector = Selector::parse("td.data a").unwrap();
                let link_selector = Selector::parse("td.data a").unwrap();
                let school_selector = Selector::parse("td.data:nth-child(3)").unwrap();
                let document = scraper::Html::parse_document(&html);
                for row_node in document.select(&row_selector) {
                    // Extract data from each row
                    let subject_area_course_code =
                        extract_text(row_node.select(&code_selector).next().unwrap());
                    let subject_area_course_name =
                        extract_text(row_node.select(&name_selector).next().unwrap());
                    let url = get_html_link_to_page(
                        row_node
                            .select(&link_selector)
                            .next()
                            .map_or("", |node| node.value().attr("href").unwrap_or("")),
                    );
                    let school = extract_text(row_node.select(&school_selector).next().unwrap());
                    let page = SchoolAreaPage {
                        subject_area_course_code,
                        subject_area_course_name,
                        school,
                        subject_area_scraper: SubjectAreaScraper::new(url),
                    };

                    self.pages.push(page);
                }
                
                Ok(())
            }
            None => Err(Box::new(UrlInvalidError)),
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
