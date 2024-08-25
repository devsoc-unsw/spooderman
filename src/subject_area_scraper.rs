use scraper::Selector;

use crate::{
    class_scraper::ClassScraper,
    scraper::fetch_url,
    text_manipulators::{extract_text, get_html_link_to_page},
    Scraper, UrlInvalidError,
};

#[derive(Debug)]
pub struct SubjectAreaScraper {
    pub url: Option<String>,
    pub class_scrapers: Vec<ClassScraper>,
}

impl Scraper for SubjectAreaScraper {
    async fn scrape(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.url {
            Some(url) => {
                let html = fetch_url(url).await?;
                let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight").unwrap();
                let code_selector = Selector::parse("td.data").unwrap();
                let name_selector = Selector::parse("td.data a").unwrap();
                let link_selector = Selector::parse("td.data a").unwrap();
                let uoc_selector = Selector::parse("td.data:nth-child(3)").unwrap();
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
                    let uoc = extract_text(row_node.select(&uoc_selector).next().unwrap())
                        .parse()
                        .expect("Could not parse UOC!");
                    self.class_scrapers.push(ClassScraper {
                        subject_area_course_code,
                        subject_area_course_name,
                        uoc,
                        url,
                    });
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
