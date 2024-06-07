use log::info;
use scraper::{Html, Selector};
use select::document;

use crate::{
    scraper::{fetch_url},
    text_manipulators::{extract_text, get_html_link_to_page},
    Scraper, UrlInvalidError,
};

#[derive(Debug)]
pub enum Career {
    UG,
    PG,
    RESEARCH,
}

#[derive(Debug)]
pub enum Term {
    T1,
    T2,
    T3,
    Summer,
}

#[derive(Debug)]
pub struct Course {
    subject_area_course_code: String,
    subject_area_course_name: String,
    uoc: i32,
    faculty: Option<String>,
    school: Option<String>,
    campus: Option<String>,
    career: Option<String>,
    terms: Vec<Term>,
    census_dates: Vec<String>,
    classes: Vec<Class>,
    notes: Option<String>,
}


#[derive(Debug)]
enum Status {
    Open,
    Closed,
}

#[derive(Debug)]
struct Enrolment {
    enrolled: u32,
    capacity: u32,
}

#[derive(Debug)]
pub struct Class {
    class_id: u32,
    section: String,
    term: Term,
    activity: String,
    status: Status,
    course_enrolment: Enrolment,
    term_date: String,
    mode: String,
    times: String,
}

#[derive(Debug)]
pub struct ClassScraper {
    pub subject_area_course_code: String,
    pub subject_area_course_name: String,
    pub uoc: i32,
    pub url: String,
}


impl Scraper for ClassScraper {
    async fn scrape(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let html = fetch_url(&self.url).await?;

        let document = scraper::Html::parse_document(&html);

        let form_bodies = Selector::parse("td.formBody td.formBody").unwrap();
        let information_body = document.select(&form_bodies).skip(0).next().unwrap();
        let table_selector = Selector::parse("td.formBody > table:nth-of-type(1) > tbody > tr").unwrap();
        let label_selector = Selector::parse("td.label").unwrap();
        let data_selector = Selector::parse("td.data").unwrap();
        let mut course_info = Course {
            subject_area_course_code: self.subject_area_course_code.clone(),
            subject_area_course_name: self.subject_area_course_name.clone(),
            uoc: self.uoc,
            faculty: None,
            school: None,
            campus: None,
            career: None,
            terms: vec![],
            census_dates: vec![],
            classes: vec![],
            notes: None
        };
        
        for row in information_body.select(&table_selector) {
            let labels: Vec<_> = row.select(&label_selector).map(|el| el.text().collect::<Vec<_>>().join("")).collect();
            let data: Vec<_> = row.select(&data_selector).map(|el| el.text().collect::<Vec<_>>().join("")).collect();

            // Print or process the extracted labels and data
            for (label, data) in labels.iter().zip(data.iter()) {
                // println!("Label: {}, Data: {}", label, data);
                
                match label.trim().to_lowercase().as_str() {
                    "faculty" => course_info.faculty = Some(data.clone()),
                    "school" => course_info.school = Some(data.clone()),
                    "campus" => course_info.campus = Some(data.clone()),
                    "career" => course_info.career = Some(data.clone()),
                    _ => {}
                }
            }

        }
        
        // let term_course_information_table = Selector::parse("td.formBody td.formBody table:nth-of-type(3) tbody tr").unwrap();
        let term_course_information_table = Selector::parse("td.formBody td.formBody table:nth-of-type(3) tbody").unwrap();
        
        let valid_row_data_len = 1;
        for row in document.select(&term_course_information_table) {
            let cell_selector = Selector::parse("td.data").unwrap();
            let cells: Vec<_> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<Vec<_>>().join("").trim().replace("\u{a0}", ""))
                .filter(|text| !text.is_empty())
                .collect();
            if cells.len() <= valid_row_data_len {
                continue;
            }
            
            let duplicate_term_removed = cells[1..].to_vec();

            // println!("{:?}", duplicate_term_removed);
        }

        // let term_course_information_table = Selector::parse("td.formBody td.formBody table:nth-of-type(3) tbody tr").unwrap();
        let term_course_information_table = Selector::parse("td.formBody td.formBody table").unwrap();

        let term_count = 3;
        let skip_count = 3 + term_count + 3 * term_count; //

        for row in document.select(&term_course_information_table).skip(skip_count) {
            let cell_selector = Selector::parse("*").unwrap();
            let mut cells: Vec<_> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>().trim().replace("\u{a0}", ""))
                .flat_map(|line| line.split('\n').filter(|text| !text.is_empty()).map(String::from).collect::<Vec<_>>())
                .collect();
            cells.iter_mut().for_each(|s| *s = s.trim().to_string());
            
            println!("{:?}", cells) ;
        }
        
        // let label_selector = Selector::parse("td.label").unwrap();
        // let data_selector = Selector::parse("td.data").unwrap();
        // let font_selector = Selector::parse("font").unwrap();
        // let row_selector = Selector::parse("tr.rowHighlight, tr.rowLowlight").unwrap();

        // let valid_row_data_len = 1;
    //     let mut data = Vec::new();
    //     let labels = document.select(&label_selector);
    //     for label in labels {
    //         if let Some(next_data) = label.next_sibling().and_then(|sibling| sibling.value().as_text()) {
    //             let value = next_data.trim().replace("\u{a0}", "");
    //             if !value.is_empty() {
    //                 data.push(value);
    //             }
    //         }
    //     }
    //     // Handle font inside data cells
    // for data_cell in document.select(&data_selector) {
    //     let text = data_cell.text().collect::<Vec<_>>().concat().trim().replace("\u{a0}", "");
    //     if !text.is_empty() {
    //         if let Some(font) = data_cell.select(&font_selector).next() {
    //             let font_text = font.text().collect::<Vec<_>>().concat().trim().replace("\u{a0}", "");
    //             if !font_text.is_empty() {
    //                 data.push(font_text);
    //             } else {
    //                 data.push(text);
    //             }
    //         } else {
    //             data.push(text);
    //         }
    //     }
    // }

    // // Extracting meeting information
    // let mut meeting_info = Vec::new();
    // for row in document.select(&row_selector) {
    //     let mut row_data = Vec::new();
    //     for cell in row.select(&data_selector) {
    //         let text = cell.text().collect::<Vec<_>>().concat().trim().replace("\u{a0}", "");
    //         row_data.push(text);
    //     }
    //     meeting_info.push(row_data);
    // }
    //   // Printing extracted data
    // //   println!("Extracted Data: {:?}", data);
    //   println!("Meeting Information: {:?}", meeting_info);
  

        Ok(())
    }
}




// impl ClassScraper {
//     pub fn new(url: String) -> Self {
        // ClassScraper {
        //     subject_area_course_code: todo!(),
        //     subject_area_course_name: todo!(),
        //     uoc: todo!(),
        //     url,
        // }
//     }
// }
