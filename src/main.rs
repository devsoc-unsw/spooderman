use chrono::Datelike;
use dotenv::dotenv;
use futures::future::join_all;
use serde_json::{json, to_writer_pretty};
use spooderman::{
    mutate_string_to_include_curr_year, send_batch_data, Class, Course, SchoolAreaScraper, Time
};
use spooderman::{ReadFromFile, ReadFromMemory};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;
use std::vec;
extern crate env_logger;
extern crate log;

use log::warn;
use log::LevelFilter;

async fn run_all_school_offered_courses_scraper_job(curr_year: i32) -> Option<SchoolAreaScraper> {
    match std::env::var("TIMETABLE_API_URL") {
        Ok(url) => {
            let url_to_scrape = mutate_string_to_include_curr_year(&mut url.to_string(), curr_year.to_string());
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
    let mut tasks = vec![];

    // Iterate over the pages and create tasks for each scrape operation
    for school_area_scrapers in &mut all_school_offered_courses_scraper.pages {
        let scraper = Arc::clone(&school_area_scrapers.subject_area_scraper);
        let task = tokio::spawn(async move {
            let mut scraper = scraper.lock().await;
            let _ = scraper.scrape().await;
        });
        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        let _ = task.await;
    }
}

use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

async fn run_course_classes_page_scraper_job(
    all_school_offered_courses_scraper: &mut SchoolAreaScraper,
) -> Vec<Course> {
    let mut tasks = vec![];
    let semaphore = Arc::new(Semaphore::new(80)); // no of concurrent tasks
    let rate_limit_delay = Duration::from_millis(1); // delay between tasks

    for school_area_scrapers in &mut all_school_offered_courses_scraper.pages {
        let scraper = Arc::clone(&school_area_scrapers.subject_area_scraper);

        // Lock the mutex to access the underlying data
        let class_scrapers = {
            let scraper = scraper.lock().await;
            scraper.class_scrapers.clone()
        };

        for class_area_scraper in class_scrapers {
            let class_area_scraper = Arc::clone(&class_area_scraper); // Clone the Arc
            let semaphore = Arc::clone(&semaphore); // Clone the semaphore

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap(); // Acquire a permit before starting a task

                // Rate limiting: wait for a bit before starting the task
                sleep(rate_limit_delay).await;

                // Perform the scraping task
                class_area_scraper
                    .lock()
                    .await
                    .scrape()
                    .await
                    .map_err(|e| Box::new(e) as Box<dyn Error + Send>)
            });

            tasks.push(task);
        }
    }

    // Wait for all tasks to complete and collect results
    let results: Vec<Result<Course, Box<dyn Error + Send>>> = join_all(tasks)
        .await
        .into_iter()
        .map(|result| result.unwrap_or_else(|e| Err(Box::new(e) as Box<dyn Error + Send>))) // Handle errors
        .collect();

    // Filter out errors and collect successful results
    let courses_vec: Vec<Course> = results.into_iter().filter_map(Result::ok).collect();

    courses_vec
}

fn convert_courses_to_json(course_vec: &mut Vec<Course>) -> Vec<serde_json::Value> {
    let mut json_courses = Vec::new();
    for course in course_vec.iter() {
        json_courses.push(json!({
            "course_code": course.course_code,
            "course_name": course.course_name,
            "uoc": course.uoc,
            "faculty": course.faculty,
            "school": course.school,
            "campus": course.campus,
            "career": course.career,
            "terms": json![course.terms],
            "modes": course.modes.iter().collect::<Vec<_>>(),
        }));
    }

    json_courses
}
fn generate_time_id(class: &Class, time: &Time) -> String {
    class.class_id.to_string() + &time.day + &time.location + &time.time + &time.weeks
}
fn convert_classes_times_to_json(course_vec: &mut Vec<Course>) -> Vec<serde_json::Value> {
    let mut times_json = Vec::<serde_json::Value>::new();
    for course in course_vec.iter() {
        for class in course.classes.iter() {
            if class.times.is_some() {
                for time in class.times.as_ref().unwrap().into_iter() {
                    times_json.push(json!({
                        "id": generate_time_id(class, time),
                        "class_id": class.class_id,
                        "day": time.day,
                        "instructor": time.instructor,
                        "location": time.location,
                        "time": time.time,
                        "weeks": time.weeks,
                    }));
                }
            }
        }
    }

    times_json
}
fn convert_classes_to_json(course_vec: &mut Vec<Course>) -> Vec<serde_json::Value> {
    let mut json_classes = Vec::new();
    for course in course_vec.iter() {
        for class in course.classes.iter() {
            json_classes.push(json!({
                "course_id": class.course_id,
                "class_id": class.class_id,
                "section": class.section,
                "term": class.term,
                "year": class.year,
                "activity": class.activity,
                "status": class.status,
                "course_enrolment": class.course_enrolment,
                "offering_period": class.offering_period,
                "meeting_dates": class.meeting_dates,
                "census_date": class.census_date,
                "consent": class.consent,
                "mode": class.mode,
                "class_notes": class.class_notes,
            }));
        }
    }

    json_classes
}

async fn handle_scrape(course_vec: &mut Vec<Course>, start_year: i32) -> Result<(), Box<dyn Error>> {
    for year in &[2025] { // TODO: Batch the 2024 and 2025 years out since both too big to insert into hasura
        println!("Handling scrape for year: {year}");
        let mut all_school_offered_courses_scraper = run_all_school_offered_courses_scraper_job(*year).await;
        if let Some(all_school_offered_courses_scraper) = &mut all_school_offered_courses_scraper {
            run_school_courses_page_scraper_job(all_school_offered_courses_scraper).await;
            let course = run_course_classes_page_scraper_job(all_school_offered_courses_scraper).await;
            course_vec.extend(course);
        }
        // let mut rc = ClassScraper { course_code: "COMP6420".to_string(), course_name: "Hardware Security".to_string(), career: "Undergraduate".to_string(), uoc: 6, url: "https://timetable.unsw.edu.au/2025/COMP1511.html".to_string() };
        // rc.scrape().await;
        // println!("{:?}", rc);

    }
   
    Ok(())
}
async fn handle_scrape_write_to_file() -> Result<(), Box<dyn Error>> {
    let mut course_vec: Vec<Course> =  Vec::<Course>::new();
    let current_year = chrono::Utc::now().year();
    handle_scrape(&mut course_vec, current_year)
            .await
            .expect("Something went wrong with scraping!");
    println!("Writing to disk!");
    let json_classes = convert_classes_to_json(&mut course_vec);
    let json_courses = convert_courses_to_json(&mut course_vec);
    let json_times = convert_classes_times_to_json(&mut course_vec);

    let file_classes = File::create("classes.json")?;
    let file_courses = File::create("courses.json")?;
    let file_times = File::create("times.json")?;
    to_writer_pretty(file_classes, &json_classes)?;
    to_writer_pretty(file_courses, &json_courses)?;
    to_writer_pretty(file_times, &json_times)?;
    Ok(())
}

async fn handle_batch_insert() -> Result<(), Box<dyn Error>> {
    println!("Handling batch insert...");
    if !Path::new("courses.json").is_file() {
        return Err(Box::new(std::io::Error::new(
            ErrorKind::NotFound,
            "courses.json doesn't exist, please run cargo r -- scrape".to_string(),
        )));
    }
    if !Path::new("classes.json").is_file() {
        return Err(Box::new(std::io::Error::new(
            ErrorKind::NotFound,
            "classes.json doesn't exist, please run cargo r -- scrape".to_string(),
        )));
    }
    if !Path::new("times.json").is_file() {
        return Err(Box::new(std::io::Error::new(
            ErrorKind::NotFound,
            "times.json doesn't exist, please run cargo r -- scrape".to_string(),
        )));
    }

    let _ = send_batch_data(&ReadFromFile).await;
    Ok(())
}

async fn handle_scrape_n_batch_insert() -> Result<(), Box<dyn Error>> {
    println!("Handling scrape and batch insert...");
    let mut course_vec: Vec<Course> =  Vec::<Course>::new();
    let current_year = chrono::Utc::now().year();
    handle_scrape(&mut course_vec, current_year)
            .await
            .expect("Something went wrong with scraping!");
    let json_classes = convert_classes_to_json(&mut course_vec);
    let json_courses = convert_courses_to_json(&mut course_vec);
    let json_times = convert_classes_times_to_json(&mut course_vec);
    let rfm = ReadFromMemory {
        courses_vec: json_courses,
        classes_vec: json_classes,
        times_vec: json_times,
    };
    let _ = send_batch_data(&rfm).await;
    Ok(())
}

fn print_help() {
    println!("Usage:");
    println!("  scrape                - Perform scraping. Creates a json file to store the data.");
    println!("  scrape_n_batch_insert - Perform scraping and batch insert. Does not create a json file to store the data.");
    println!("  batch_insert          - Perform batch insert on json files created by scrape.");
    println!("  help                  - Show this help message");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::Builder::new()
        .filter_level(LevelFilter::Error)
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [options]", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    match command.as_str() {
        "scrape" => handle_scrape_write_to_file().await?,
        "scrape_n_batch_insert" => handle_scrape_n_batch_insert().await?,
        "batch_insert" => handle_batch_insert().await?,
        "help" => print_help(),
        _ => {
            eprintln!("Unknown command: '{}'", command);
            print_help();
            std::process::exit(1);
        }
    }

    Ok(())
}
