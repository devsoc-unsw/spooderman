use dotenv::dotenv;
use serde_json::{json, to_writer_pretty};
use spooderman::{
    mutate_string_to_include_curr_year, send_batch_data, Course, SchoolAreaScraper, Scraper,
};
use std::env;
use std::error::Error;
use std::fs::File;
use std::vec;
extern crate env_logger;
extern crate log;

use log::warn;
use log::LevelFilter;

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

fn convert_classes_times_to_json(course_vec: &mut Vec<Course>) -> Vec<serde_json::Value> {
    let mut times_json = Vec::<serde_json::Value>::new();
    for course in course_vec.iter() {
        for class in course.classes.iter() {
            if class.times.is_some() {
                for time in class.times.as_ref().unwrap().into_iter() {
                    times_json.push(json!({
                        "class_id": class.class_id,
                        "course_id": class.course_id,
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

async fn handle_scrape() -> Result<(), Box<dyn Error>> {
    println!("Handling scrape...");
    let mut course_vec: Vec<Course> = vec![];
    let mut all_school_offered_courses_scraper = run_all_school_offered_courses_scraper_job().await;
    if let Some(all_school_offered_courses_scraper) = &mut all_school_offered_courses_scraper {
        run_school_courses_page_scraper_job(all_school_offered_courses_scraper).await;
        let course = run_course_classes_page_scraper_job(all_school_offered_courses_scraper).await;
        course_vec.extend(course);
    }
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
    let _ = send_batch_data().await;
    Ok(())
}

async fn handle_scrape_n_batch_insert() -> Result<(), Box<dyn Error>> {
    println!("Handling scrape and batch insert...");
    let _ = handle_scrape().await;
    let _ = handle_batch_insert().await;
    Ok(())
}

fn print_help() {
    println!("Usage:");
    println!("  scrape                - Perform scraping");
    println!("  scrape_n_batch_insert - Perform scraping and batch insert");
    println!("  batch_insert          - Perform batch insert. Note if the json files dont exist, it will be created!");
    println!("  help                  - Show this help message");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <command> [options]", args[0]);
        std::process::exit(1);
    }

    let command = &args[1];
    match command.as_str() {
        "scrape" => handle_scrape().await?,
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
