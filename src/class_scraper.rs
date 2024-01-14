use scraper::Selector;

use crate::{
    scraper::{fetch_url, ClassTimeBlock, Course, DateBlock, Enrolment, Page, Status, Term},
    text_manipulators::{extract_text, get_html_link_to_page},
    Scraper, UrlInvalidError,
};

#[derive(Debug)]
pub struct ClassPage {
    url: String,
    subject_area_course_code: String,
    subject_area_course_name: String,
    school: String,
    courses: Vec<Course>,
}

#[derive(Debug)]
pub struct Class {
    class_id: u32,
    section: String,
    term: Term,
    activity: String,
    status: Status,
    course_enrolment: Enrolment,
    term_date: DateBlock,
    mode: String,
    times: Vec<ClassTimeBlock>,
}

impl Page for ClassPage {
    fn view_page_details(&self) {
        println!("{:?}", self)
    }
}

#[derive(Debug)]

pub struct ClassScraper {
    pub url: Option<String>,
    pub pages: Vec<Box<dyn Page>>,
}

impl std::fmt::Debug for dyn Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.view_page_details())
    }
}

impl ClassScraper {
    pub async fn run_scraper_on_url(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.url {
            Some(url) => {
                let html = fetch_url(url).await?;
                // println!("{}", html);
                let form_body_selector =
                    Selector::parse("td.formBody tbody tr td.formBody").unwrap();
                let course_summary = Selector::parse("tbody tr").unwrap();

                // let name_selector = Selector::parse("td.data a").unwrap();
                // let link_selector = Selector::parse("td.data a").unwrap();
                // let school_selector = Selector::parse("td.data:nth-child(3)").unwrap();
                let document = scraper::Html::parse_document(&html);
                for td in document.select(&form_body_selector) {
                    // Selector for tables within the td element
                    let table_selector = Selector::parse("table").unwrap();

                    // Iterate over tables
                    for table in td.select(&table_selector) {
                        // Selector for rows within the table
                        let row_selector = Selector::parse("tr").unwrap();

                        // Iterate over rows
                        for row in table.select(&row_selector) {
                            // Selector for cells within the row
                            let cell_selector = Selector::parse("td").unwrap();

                            // Extract and print data from each cell
                            for cell in row.select(&cell_selector) {
                                println!("{}", cell.text().collect::<String>());
                            }
                        }
                    }
                }
                // for row_node in document.select(&form_body_selector) {
                //     let course_summary = extract_text(row_node.select(&course_summary).next().unwrap());
                //     println!("{:?}", course_summary);
                // //     // Extract data from each row
                // //     let subject_area_course_code =
                // //         extract_text(row_node.select(&code_selector).next().unwrap());
                // //     let subject_area_course_name =
                // //         extract_text(row_node.select(&name_selector).next().unwrap());
                // //     let url = get_html_link_to_page(
                // //         row_node
                // //             .select(&link_selector)
                // //             .next()
                // //             .map_or("", |node| node.value().attr("href").unwrap_or("")),
                // //     );
                // //     let school = extract_text(row_node.select(&school_selector).next().unwrap());
                // //     // Create a Course struct and push it to the vector
                // //     let page = SubjectAreaPage {
                // //         subject_area_course_code,
                // //         subject_area_course_name,
                // //         url,
                // //         school,
                // //         courses: Vec::new(),
                // //     };

                // //     self.add_page(Box::new(page));
                // }

                println!("{:?}", self.pages);
                Ok(())
            }
            None => Err(Box::new(UrlInvalidError)),
        }
    }
}
impl Scraper for ClassScraper {
    fn new() -> Self {
        ClassScraper {
            url: None,
            pages: Vec::new(),
        }
    }

    fn set_url(&mut self, url: String) -> Self {
        ClassScraper {
            url: Some(url),
            pages: Vec::new(),
        }
    }

    fn add_page(&mut self, page: Box<dyn Page>) {
        self.pages.push(page);
    }
}
