use spooderman::{ClassScraper, SchoolAreaScraper, Scraper, SubjectAreaScraper};
use dotenv::dotenv;
use regex::Regex;
use chrono::{Datelike, Utc};
use std::collections::HashMap;
use futures::stream::{StreamExt, FuturesUnordered};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::error::Error;


extern crate log;
extern crate env_logger;

use log::LevelFilter;

use log::{info, warn, error};

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
    pattern.replace(&curr_base_url, Utc::now().year().to_string()).to_string()
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

async fn run_school_courses_page_scraper_job() -> SchoolAreaScraper { 
    // if let Some(school_area_scrapers) = all_school_offered_courses_scraper.as_mut() {
    //     for school_area_page in &mut school_area_scrapers.pages {
    //         let _ = school_area_page.subject_area_scraper.scrape().await;
    //         println!("TEEHEE");
    //         for mut class_scrapers in &mut school_area_page.subject_area_scraper.class_scrapers {

    //         }
    //         // Construct Courses from here and transfer ownership
            
    //     }
    // }
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();
    let class_vec = vec![];
    let course_vec: vec![];
    let mut all_school_offered_courses_scraper = run_all_school_offered_courses_scraper_job().await;
    
    println!("{:?}", all_school_offered_courses_scraper);

    
        // let scraping_jobs = FuturesUnordered::new();
        // let mut all_school_offered_courses_scraper = run_all_school_offered_courses_scraper_job().await;
        
        // for mut scraper in all_school_offered_courses_scraper {
        //     scraping_jobs.push(async { scraper.scrape().await });
        // }

        // scraping_jobs.for_each_concurrent(None, |scrape_future| async {
        //     scrape_future;
        // }).await;
        // println!("{:?}", all_school_offered_courses_scraper);
    // // The base API url needs the year to be replaced with "year".
    // // ie. https://timetable.unsw.edu.au/year/subjectSearch.html
    // match std::env::var("TIMETABLE_API_URL") {
    //     Ok(url) => {
    //         info!("Timetable URL has been parsed from environment file: {url}!");
    //         let url_to_scrape = mutate_string_to_include_curr_year(&mut url.to_string());
            
    //         let mut scraper = ClassScraper {
    //             subject_area_course_code: "COMP1511".to_string(),
    //             subject_area_course_name: "COMP1511".to_string(),
    //             uoc: 6,
    //             // url: "https://timetable.unsw.edu.au/2024/ACCT5999.html".to_string(), 
    //             url: "https://timetable.unsw.edu.au/2024/COMP1511.html".to_string(),
    //         };
    //         let _ = scraper.scrape().await;
    //         // let mut scraper = SchoolAreaScraper::new(url_to_scrape);
            
    //         // // let mut scraper = SubjectAreaScraper::new("https://timetable.unsw.edu.au/2024/COMPKENS.html".to_string());
    //         // match scraper.scrape().await {
    //         //     Ok(_) => info!("Scraping successful!\n"),
    //         //     Err(e) => error!("Error: {}", e),
    //         // } 
    //         // for school_area_page in &mut scraper.pages {
    //         //     let _ = school_area_page.subject_area_scraper.scrape().await;
    //         // }
    //         // println!("{:?}", scraper);


    //     }
    //     Err(e) => {
    //         warn!("Timetable URL has NOT been parsed properly from env file and error report: {e}");
    //     }
    // };

}
