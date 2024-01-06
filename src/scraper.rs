use chrono::{DateTime, Utc};
use reqwest::ClientBuilder;
use scraper::{html, ElementRef, Selector};
use std::ops::Add;

use crate::{UrlInvalidError, subject_area_scraper::SubjectAreaPage};

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


pub trait Page {
    fn view_page_details(&self);
}
pub trait Scraper {
    fn new() -> Self;
    fn set_url(&mut self, url: String) -> Self;
    fn add_page(&mut self, page: Box::<dyn Page>);
}

pub async fn fetch_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    Ok(body)
}
// impl Scraper {
    

    

//     pub fn add_page(&mut self, page: impl Page) {
//         self.pages.push(Box::new(page));
//     }

//     // pub async fn run_scraper(&mut self) -> Result<(), Box<dyn std::error::Error>> {
//     //     self.subject_area_scrape().await
//     // }
// }

// impl Scraper {
//     pub fn view_scraper(&self) {
//         println!("{:?}", self);
//     }
// }

// impl Default for Scraper {
//     fn default() -> Self {
//         Self::new()
//     }
// }

