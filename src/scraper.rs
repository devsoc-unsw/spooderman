use chrono::{DateTime, Utc};
use reqwest::ClientBuilder;
use scraper::{html, ElementRef, Selector};
use std::ops::Add;

use crate::UrlInvalidError;

#[derive(Debug)]
enum Term {
    T1,
    T2,
    T3,
    Summer,
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
struct TimeBlock {
    start: (u32, u32),
    end: (u32, u32),
}

impl Add for TimeBlock {
    type Output = TimeBlock;

    fn add(self, another: TimeBlock) -> Self {
        let add_hours = |a, b| (a + b) % 24;
        let add_minutes = |a, b| (a + b) % 60;
        Self {
            start: (
                add_hours(self.start.0, another.start.0),
                add_minutes(self.start.1, another.start.1),
            ),
            end: (
                add_hours(self.end.0, another.end.0),
                add_minutes(self.end.1, another.end.1),
            ),
        }
    }
}

#[derive(Debug)]
struct DateBlock {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Debug)]
enum Day {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

#[derive(Debug)]
pub struct ClassTimeBlock {
    day: Day,
    weeks: String,
    time: TimeBlock,
    location: String,
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

#[derive(Debug)]
enum Career {
    UG,
    PG,
    RESEARCH,
}

#[derive(Debug)]
pub struct Course {
    code: String,
    name: String,
    campus: Career,
    career: String,
    terms: Vec<Term>,
    census_dates: Vec<String>,
    classes: Vec<Class>,
    notes: String,
}

#[derive(Debug)]
pub struct Page {
    url: String,
    subject_area_course_code: String,
    subject_area_course_name: String,
    school: String,
    courses: Vec<Course>,
}

#[derive(Debug)]
pub struct Scraper {
    url: Option<String>,
    pages: Option<Vec<Page>>,
}

impl Scraper {
    pub fn new() -> Self {
        Scraper {
            url: None,
            pages: Some(Vec::new()),
        }
    }

    pub fn set_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn add_page(mut self, page: Page) -> Self {
        let mut new_pages = self.pages.take().unwrap_or_default();
        new_pages.push(page);
        self.pages = Some(new_pages);
        self
    }

    async fn fetch_url(&self, url: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()?;
        let response = client.get(url).send().await?;
        let body = response.text().await?;
        Ok(body)
    }

    pub async fn run_scraper(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match &self.url {
            Some(url) => {
                let html = self.fetch_url(url).await?;
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
                    let page = Page {
                        subject_area_course_code,
                        subject_area_course_name,
                        url,
                        school,
                        courses: Vec::new(),
                    };

                    match &mut self.pages {
                        Some(curr_pages) => {
                            curr_pages.push(page);
                        }
                        None => {
                            self.pages = Some(vec![page]);
                        }
                    }
                }

                println!("{:?}", self.pages);
                Ok(())
            }
            None => Err(Box::new(UrlInvalidError)),
        }
    }
}

impl Scraper {
    pub fn view_scraper(&self) {
        println!("{:?}", self);
    }
}

impl Default for Scraper {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_text(node: ElementRef) -> String {
    node.text().collect::<String>()
}

fn get_html_link_to_page(html_fragment: &str) -> String {
    "https://timetable.unsw.edu.au/2024/".to_string() + html_fragment
}
