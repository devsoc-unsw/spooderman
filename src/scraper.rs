use std::ops::Add;
use chrono::{DateTime, Utc};
use reqwest::ClientBuilder;
use scraper::Html;

use crate::UrlInvalidError;


#[derive(Debug)]
enum Term {
    T1, T2, T3, Summer
}

#[derive(Debug)]
enum Status { Open, Closed }

#[derive(Debug)]
struct Enrolment { enrolled: u32, capacity: u32 }

#[derive(Debug)]
struct TimeBlock { start: (u32, u32), end: (u32, u32) }

impl Add for TimeBlock {
    type Output = TimeBlock;
    
    fn add(self, another: TimeBlock) -> Self {
        let add_hours = |a, b| (a + b) % 24;
        let add_minutes = |a, b| (a + b) % 60;
        Self {
            start: (add_hours(self.start.0, another.start.0), add_minutes(self.start.1, another.start.1)),
            end: (add_hours(self.end.0, another.end.0), add_minutes(self.end.1, another.end.1))
        }
    }
}

#[derive(Debug)]
struct DateBlock { start: DateTime<Utc>, end: DateTime<Utc> }


#[derive(Debug)]
enum Day { 
  Sunday, Monday, Tuesday, Wednesday, Thursday, Friday, Saturday
}

#[derive(Debug)]
struct ClassTimeBlock { 
  day: Day, 
  weeks: String, 
  time: TimeBlock,
  location: String,
}


#[derive(Debug)]
struct Class {
    class_id: u32,
    section: String,
    term: Term,
    activity: String,
    status: Status,
    course_enrolment: Enrolment,
    term_date: DateBlock,
    mode: String,
    times: Vec<ClassTimeBlock>
}

#[derive(Debug)]
enum Career { UG, PG }

#[derive(Debug)]
struct Course {
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
struct Page {
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
            pages: None,
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
      let client = ClientBuilder::new().danger_accept_invalid_certs(true).build()?;
      let response = client.get(url).send().await?;
      let body = response.text().await?;
      Ok(body)
    }

  pub async fn run_scraper(&mut self) -> Result<Html, Box<dyn std::error::Error>> {
      match &self.url { 
        Some(url) => {
          let html = self.fetch_url(url).await?;
          println!("{}", html);
          let html_course_selector = scraper::Selector::parse("tr.rowLowlight td.data").unwrap();
          let doc = scraper::Html::parse_document(&html);
          let res: Vec<_> = doc.select(&html_course_selector).flat_map(|el| el.text()).collect();
          println!("{:?}", res);
          Ok(doc)
        }
        None => {
          Err(Box::new(UrlInvalidError))
        }
      }
      
  }
}

impl Scraper {
    pub fn view_scraper(&self) {
        println!("{:?}", self);
    }
}