use std::ops::Add;
use chrono::{DateTime, Utc};

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
    times: Vec<Box<ClassTimeBlock>>
}

#[derive(Debug)]
enum Career { UG, PG }

#[derive(Debug)]
struct Course {
    code: String,
    name: String,
    campus: Career,
    career: String,
    terms: Vec<Box<Term>>,
    census_dates: Vec<Box<String>>,
    classes: Vec<Box<Class>>,
    notes: String,
}

#[derive(Debug)]
struct Page {
  url: String,
  subject_area_course_code: String,
  subject_area_course_name: String,
  school: String,
  courses: Vec<Box<Course>>,
}


#[derive(Debug)]
struct Scraper {
  url: String,
  pages: Vec<Box<Page>>,
}



