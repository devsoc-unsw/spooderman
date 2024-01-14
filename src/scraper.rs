use chrono::{DateTime, Utc};
use reqwest::ClientBuilder;
use tokio::sync::Mutex;
use std::ops::Add;

use crate::class_scraper::Class;


// static COURSE_LIST: Mutex<HashMap<String, >>

#[derive(Debug)]
pub enum Term {
    T1,
    T2,
    T3,
    Summer,
}

#[derive(Debug)]
pub enum Status {
    Open,
    Closed,
}

#[derive(Debug)]
pub struct Enrolment {
    enrolled: u32,
    capacity: u32,
}

#[derive(Debug)]
pub struct TimeBlock {
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
pub struct DateBlock {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[derive(Debug)]
pub enum Day {
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
enum Career {
    UG,
    PG,
    RESEARCH,
    OTHER,
}

#[derive(Debug)]
pub struct Course {
    url: String,
    code: String,
    name: String,
    uoc: u8,
    campus: String,
    school: String,
    career: Career,
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
    fn add_page(&mut self, page: Box<dyn Page>);
}

pub async fn fetch_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    Ok(body)
}
