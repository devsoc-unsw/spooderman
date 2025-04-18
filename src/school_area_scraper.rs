use crate::{
    Course,
    requests::RequestClient,
    subject_area_scraper::SubjectArea,
    text_manipulators::{extract_text, extract_year, get_html_link_to_page},
};
use derive_new::new;
use scraper::Selector;
use std::sync::Arc;

#[derive(Debug, new)]
pub struct SchoolAreaPage {
    pub course_code: String,
    pub course_name: String,
    pub school: String,
    pub subject_area: SubjectArea,
}

#[derive(Debug)]
pub struct SchoolArea {
    pub url: String,
    pub pages: Vec<SchoolAreaPage>,
}

impl SchoolArea {
    pub async fn scrape(url: String, request_client: &Arc<RequestClient>) -> anyhow::Result<Self> {
        log::info!("Scraping School Area for: {}", url);

        let html = request_client.fetch_url(&url).await?;
        let document = scraper::Html::parse_document(&html);

        let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight").unwrap();
        let code_selector = Selector::parse("td.data").unwrap();
        let name_selector = Selector::parse("td.data a").unwrap();
        let link_selector = Selector::parse("td.data a").unwrap();
        let school_selector = Selector::parse("td.data:nth-child(3)").unwrap();

        let mut pages = vec![];
        for row_node in document.select(&row_selector) {
            // Extract data from each row
            let course_code = extract_text(row_node.select(&code_selector).next().unwrap());
            let course_name = extract_text(row_node.select(&name_selector).next().unwrap());
            let url_to_scrape_further = get_html_link_to_page(
                extract_year(&url).unwrap() as i32,
                row_node
                    .select(&link_selector)
                    .next()
                    .map_or("", |node| node.value().attr("href").unwrap_or("")),
            );
            let school = extract_text(row_node.select(&school_selector).next().unwrap());

            let subject_area = SubjectArea::scrape(url_to_scrape_further, request_client).await?;
            let subject_area_page =
                SchoolAreaPage::new(course_code, course_name, school, subject_area);
            pages.push(subject_area_page);
        }
        Ok(Self { url, pages })
    }

    pub fn get_all_courses(self) -> impl Iterator<Item = Course> {
        self.pages
            .into_iter()
            .map(|school_area_page| school_area_page.subject_area.courses)
            .flatten()
    }
}
