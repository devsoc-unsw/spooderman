use chrono::{Datelike, Utc};
use dotenv::dotenv;
use futures::stream::{FuturesUnordered, StreamExt};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_writer_pretty};
use spooderman::{ClassScraper, Course, SchoolAreaScraper, Scraper, SubjectAreaScraper};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::vec;

extern crate env_logger;
extern crate log;

use log::LevelFilter;

use log::{error, info, warn};

type CourseCode = String;
type FacultyCode = String;

#[derive(Serialize, Deserialize)]
struct Metadata {
    table_name: String,
    columns: Vec<String>,
    sql_up: String,
    sql_down: String,
    write_mode: Option<String>,
    sql_before: Option<String>,
    sql_after: Option<String>,
    dryrun: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct BatchInsertRequest {
    metadata: Metadata,
    payload: Vec<serde_json::Value>,
}

pub fn mutate_string_to_include_curr_year(curr_base_url: &mut String) -> String {
    let pattern = Regex::new("year").unwrap();
    pattern
        .replace(&curr_base_url, Utc::now().year().to_string())
        .to_string()
}

async fn send_batch_data() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    let hasuragres_url = env::var("HASURAGRES_URL")?;
    let api_key = env::var("HASURAGRES_API_KEY")?;
    let client = Client::new();
    let requests = vec![
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "courses".to_string(),
                columns: vec!["subject_area_course_code".to_string(), "subject_area_course_name".to_string(), "uoc".to_string()],
                sql_up: "CREATE TABLE Students(\"zId\" INT PRIMARY KEY, \"name\" TEXT);".to_string(),
                sql_down: "DROP TABLE Students CASCADE;".to_string(),
                write_mode: Some("overwrite".to_string()),
                sql_before: None,
                sql_after: None,
                dryrun: Some(true),
            },
            payload: vec![
                json!({"zId": 1, "name": "Student One"}),
                json!({"zId": 2, "name": "Student Two"}),
            ],
        },
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "courses".to_string(),
                columns: vec!["course_id".to_string(), "course_name".to_string()],
                sql_up: "CREATE TABLE Courses(\"course_id\" VARCHAR(8) PRIMARY KEY, \"course_name\" TEXT);".to_string(),
                sql_down: "DROP TABLE Courses CASCADE;".to_string(),
                write_mode: Some("append".to_string()),
                sql_before: None,
                sql_after: None,
                dryrun: Some(false),
            },
            payload: vec![
                json!({"course_id": "CS101", "course_name": "Introduction to Programming"}),
                json!({"course_id": "CS102", "course_name": "Data Structures"}),
            ],
        },
    ];

    let response = client
        .post(format!("{}/batch_insert", hasuragres_url))
        .header("X-API-Key", api_key)
        .json(&requests)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Batch data inserted successfully!");
    } else {
        eprintln!("Failed to insert batch data: {:?}", response.text().await?);
    }

    Ok(())
}

async fn run_all_school_offered_courses_scraper_job() -> Option<SchoolAreaScraper> {
    match std::env::var("TIMETABLE_API_URL") {
        Ok(url) => {
            let url_to_scrape = mutate_string_to_include_curr_year(&mut url.to_string());
            let mut scraper = SchoolAreaScraper::new(url_to_scrape);
            let _ = scraper.scrape().await;
            return Some(scraper);
        }
        Err(e) => {
            warn!("Timetable URL has NOT been parsed properly from env file and error report: {e}");
            return None;
        }
    }
}

async fn run_school_courses_page_scraper_job(
    all_school_offered_courses_scraper: &mut SchoolAreaScraper,
) {
    for school_area_scrapers in &mut all_school_offered_courses_scraper.pages {
        let _ = school_area_scrapers.subject_area_scraper.scrape().await;
    }
}

async fn run_course_classes_page_scraper_job(
    all_school_offered_courses_scraper: &mut SchoolAreaScraper,
) -> Vec<Course> {
    let mut courses_vec: Vec<Course> = vec![];
    for school_area_scrapers in &mut all_school_offered_courses_scraper.pages {
        for course_area_scrapers in &mut school_area_scrapers.subject_area_scraper.class_scrapers {
            let courses = course_area_scrapers.scrape().await;
            if let Ok(course) = courses {
                courses_vec.push(course);
            }
        }
    }
    return courses_vec;
}

async fn test_scrape() {
    let mut scraper = ClassScraper {
        subject_area_course_code: "COMP1511".to_string(),
        subject_area_course_name: "COMP1511".to_string(),
        uoc: 6,
        // url: "https://timetable.unsw.edu.au/2024/ACCT2101.html".to_string(),
        url: "https://timetable.unsw.edu.au/2024/COMP1511.html".to_string(),
    };
    let c = scraper.scrape().await;
    println!("{:?}", c);
    // for school_area_scrapers in &mut all_school_offered_courses_scraper.pages {
    //     for course_area_scrapers in &mut school_area_scrapers.subject_area_scraper.class_scrapers {
    //         let courses = course_area_scrapers.scrape().await;
    //         println!("{:?}", courses);
    //     }
    // }
}

fn convert_courses_to_json(course_vec: &mut Vec<Course>) -> Vec<serde_json::Value> {
    let mut json_courses = Vec::new();
    for course in course_vec.iter() {
        json_courses.push(json!({
            "subject_area_course_code": course.subject_area_course_code,
            "subject_area_course_name": course.subject_area_course_name,
            "uoc": course.uoc,
            "faculty": course.faculty,
            "school": course.school,
            "campus": course.campus,
            "career": course.career,
            "terms": course.terms,
        }));
    }

    json_courses
}

fn convert_classes_to_json(course_vec: &mut Vec<Course>) -> Vec<serde_json::Value> {
    let mut json_classes = Vec::new();
    for course in course_vec.iter() {
        for class in course.classes.iter() {
            let times_json = class.times.as_ref().map(|times| {
                let _ = times
                    .iter()
                    .map(|time| {
                        json!({
                            "day": time.day,
                            "time": time.time,
                            "location": time.location,
                            "weeks": time.weeks,
                            "instructor": time.instructor
                        })
                    })
                    .collect::<serde_json::Value>();
            });
            json_classes.push(json!({
                "class_id": class.class_id,
                "section": class.section,
                "term": class.term,
                "activity": class.activity,
                "status": class.status,
                "course_enrolment": class.course_enrolment,
                "offering_period": class.offering_period,
                "meeting_dates": class.meeting_dates,
                "census_date": class.census_date,
                "consent": class.consent,
                "mode": class.mode,
                "times": times_json,
                "class_notes": class.class_notes,
            }));
        }
    }

    json_classes
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();
    // let class_vec = vec![];
    // let mut course_vec: Vec<Course> = vec![];
    // let mut all_school_offered_courses_scraper = run_all_school_offered_courses_scraper_job().await;
    // if let Some(all_school_offered_courses_scraper) = &mut all_school_offered_courses_scraper {
    //     run_school_courses_page_scraper_job(all_school_offered_courses_scraper).await;
    //     let course = run_course_classes_page_scraper_job(all_school_offered_courses_scraper).await;
    //     println!("{:?}", course);
    //     course_vec.extend(course);
    // }

    // let mut json_classes = convert_classes_to_json(&mut course_vec);
    // let mut json_courses = convert_courses_to_json(&mut course_vec);

    // // println!("Courses: {:?}, Classes: {:?}", json_courses, json_classes);
    // // Open files for writing
    // let file_classes = File::create("classes.json")?;
    // let file_courses = File::create("courses.json")?;

    // // Write JSON to files
    // to_writer_pretty(file_classes, &json_classes)?;
    // to_writer_pretty(file_courses, &json_courses)?;
    test_scrape().await;
    Ok(())
}
