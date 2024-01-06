use scraper::Selector;

use crate::{
    scraper::{Course, Page, fetch_url},
    text_manipulators::{extract_text, get_html_link_to_page},
    Scraper, UrlInvalidError,
};

#[derive(Debug)]
pub struct SubjectAreaPage {
    url: String,
    subject_area_course_code: String,
    subject_area_course_name: String,
    school: String,
    courses: Vec<Course>,
}


impl Page for SubjectAreaPage {
    fn view_page_details(&self) {
        println!("{:?}", self)
    }
}



#[derive(Debug)]

pub struct SubjectAreaScraper {
    pub url: Option<String>,
    pub pages: Vec<Box<dyn Page>>,
}

impl std::fmt::Debug for dyn Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.view_page_details())
    }
}

impl SubjectAreaScraper {
    pub async fn run_scraper_on_url(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.url {
            Some(url) => {
                let html = fetch_url(url).await?;
                println!("{}", html);
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
                    // Create a Course struct and push it to the vector
                    let page = SubjectAreaPage {
                        subject_area_course_code,
                        subject_area_course_name,
                        url,
                        school,
                        courses: Vec::new(),
                    };

                    self.add_page(Box::new(page));
                }

                println!("{:?}", self.pages);
                Ok(())
            }
            None => Err(Box::new(UrlInvalidError)),
        }
    }
}
impl Scraper for SubjectAreaScraper {
    fn new() -> Self {
        SubjectAreaScraper {
            url: None,
            pages: Vec::new(),
        }
    }

    fn set_url(&mut self, url: String) -> Self {
        SubjectAreaScraper {
            url: Some(url),
            pages: Vec::new(),
        }
    }

    fn add_page(&mut self, page: Box::<dyn Page>) {
        self.pages.push(page);
    }
}
