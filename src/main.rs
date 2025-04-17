use anyhow::Context;
use argh::FromArgs;
use chrono::Datelike;
use dotenv::dotenv;
use enum_dispatch::enum_dispatch;
use futures::future::join_all;
use serde::Serialize;
use serde_json::{json, to_writer_pretty};
use spooderman::{
    Class, Course, SchoolAreaScraper, Time, mutate_string_to_include_curr_year, send_batch_data,
};
use spooderman::{ReadFromFile, ReadFromMemory};
use std::error::Error;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::vec;

use log::LevelFilter;

async fn run_all_school_offered_courses_scraper_job(
    curr_year: i32,
) -> anyhow::Result<SchoolAreaScraper> {
    // TODO: parse all of required env vars into a Config struct initially, and the timetable url shouldn't be optional while the hasuragres ones obviously should be.
    match std::env::var("TIMETABLE_API_URL") {
        Ok(url) => {
            let url_to_scrape =
                mutate_string_to_include_curr_year(&mut url.to_string(), curr_year.to_string());
            Ok(SchoolAreaScraper::scrape(url_to_scrape).await?)
        }
        Err(e) => Err(anyhow::anyhow!(
            "Timetable URL could NOT been parsed properly from env file and error report: {e}"
        )),
    }
}

#[derive(Debug, Serialize)]
struct Data {
    all_courses: Vec<Course>,
}

pub fn sort_by_key_ref<T, B, F>(slice: &mut [T], mut f: F)
where
    F: FnMut(&T) -> &B,
    B: Ord,
{
    slice.sort_by(|a, b| f(a).cmp(f(b)))
}

impl Data {
    fn new(mut all_courses: Vec<Course>) -> Self {
        sort_by_key_ref(&mut all_courses, |course| &course.course_id);
        Self { all_courses }
    }

    async fn write_to_single_json(&self, json_file_path: &str) -> anyhow::Result<()> {
        log::info!("Writing scraped data to {}!", json_file_path);
        let file = File::create(json_file_path)?;
        to_writer_pretty(file, &self)?;
        Ok(())
    }
}

async fn run_school_courses_page_scraper_job(
    all_school_offered_courses_scraper: &mut SchoolAreaScraper,
) {
    // Iterate over the pages and create tasks for each scrape operation
    let tasks: Vec<_> = all_school_offered_courses_scraper
        .pages
        .iter_mut()
        .map(|school_area_scrapers| {
            let scraper = Arc::clone(&school_area_scrapers.subject_area_scraper);
            tokio::spawn(async move {
                let mut scraper = scraper.lock().await;
                let _ = scraper.scrape().await;
            })
        })
        .collect();

    // Wait for all tasks to complete
    for task in tasks {
        let _ = task.await;
    }
}

use tokio::sync::Semaphore;
use tokio::time::{Duration, sleep};

async fn run_course_classes_page_scraper_job(
    all_school_offered_courses_scraper: &mut SchoolAreaScraper,
) -> Vec<Course> {
    let semaphore = Arc::new(Semaphore::new(80)); // no of concurrent tasks
    let rate_limit_delay = Duration::from_millis(1); // delay between tasks

    let mut tasks = vec![];
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

fn convert_courses_to_json(courses: &[Course]) -> Vec<serde_json::Value> {
    let mut json_courses = Vec::new();
    for course in courses.iter() {
        json_courses.push(json!({
            "course_id": course.course_id,
            "course_code": course.course_code,
            "course_name": course.course_name,
            "uoc": course.uoc,
            "faculty": course.faculty,
            "school": course.school,
            "campus": course.campus,
            "career": course.career,
            "terms": json![course.terms],
            "modes": course.modes,
        }));
    }

    json_courses
}
fn generate_time_id(class: &Class, time: &Time) -> String {
    class.class_id.to_string() + &time.day + &time.location + &time.time + &time.weeks
}
fn convert_classes_times_to_json(courses: &[Course]) -> Vec<serde_json::Value> {
    let mut times_json = Vec::<serde_json::Value>::new();
    for course in courses.iter() {
        for class in course.classes.iter() {
            if class.times.is_some() {
                for time in class.times.as_ref().unwrap().iter() {
                    times_json.push(json!({
                        "id": generate_time_id(class, time),
                        "class_id": class.class_id,
                        "day": time.day,
                        "career": time.career,
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
fn convert_classes_to_json(courses: &[Course]) -> Vec<serde_json::Value> {
    let mut json_classes = Vec::new();
    for course in courses.iter() {
        for class in course.classes.iter() {
            json_classes.push(json!({
                "course_id": class.course_id,
                "class_id": class.class_id,
                "section": class.section,
                "term": class.term,
                "career": class.career,
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

async fn handle_scrape(course_vec: &mut Vec<Course>, start_year: i32) -> anyhow::Result<()> {
    for year in &[2025] {
        // TODO: Batch the 2024 and 2025 years out since both too big to insert into hasura
        log::info!("Handling scrape for year: {year}");
        let mut all_school_offered_courses_scraper =
            run_all_school_offered_courses_scraper_job(*year).await?;
        run_school_courses_page_scraper_job(&mut all_school_offered_courses_scraper).await;
        let course =
            run_course_classes_page_scraper_job(&mut all_school_offered_courses_scraper).await;
        course_vec.extend(course);
    }

    Ok(())
}
async fn handle_scrape_write_to_file() -> anyhow::Result<()> {
    let mut course_vec: Vec<Course> = Vec::<Course>::new();
    let current_year = chrono::Utc::now().year();
    handle_scrape(&mut course_vec, current_year)
        .await
        .context("Something went wrong with scraping!")?;
    log::info!("Writing scraped data to disk!");
    let json_classes = convert_classes_to_json(&course_vec);
    let json_courses = convert_courses_to_json(&course_vec);
    let json_times = convert_classes_times_to_json(&course_vec);

    let file_classes = File::create("classes.json")?;
    let file_courses = File::create("courses.json")?;
    let file_times = File::create("times.json")?;
    to_writer_pretty(file_classes, &json_classes)?;
    to_writer_pretty(file_courses, &json_courses)?;
    to_writer_pretty(file_times, &json_times)?;
    Ok(())
}

async fn handle_batch_insert() -> anyhow::Result<()> {
    log::info!("Handling batch insert...");
    if !Path::new("courses.json").is_file() {
        return Err(anyhow::anyhow!(
            "courses.json doesn't exist, please run cargo r -- scrape"
        ));
    }
    if !Path::new("classes.json").is_file() {
        return Err(anyhow::anyhow!(
            "classes.json doesn't exist, please run cargo r -- scrape"
        ));
    }
    if !Path::new("times.json").is_file() {
        return Err(anyhow::anyhow!(
            "times.json doesn't exist, please run cargo r -- scrape"
        ));
    }

    let _ = send_batch_data(&ReadFromFile).await;
    Ok(())
}

async fn handle_scrape_n_batch_insert() -> anyhow::Result<()> {
    log::info!("Handling scrape and batch insert...");
    let mut course_vec: Vec<Course> = Vec::<Course>::new();
    let current_year = chrono::Utc::now().year();
    handle_scrape(&mut course_vec, current_year)
        .await
        .context("Something went wrong with scraping!")?;
    let json_classes = convert_classes_to_json(&course_vec);
    let json_courses = convert_courses_to_json(&course_vec);
    let json_times = convert_classes_times_to_json(&course_vec);
    let rfm = ReadFromMemory {
        courses_vec: json_courses,
        classes_vec: json_classes,
        times_vec: json_times,
    };
    let _ = send_batch_data(&rfm).await;
    Ok(())
}

#[enum_dispatch]
trait Exec {
    async fn exec(&self) -> anyhow::Result<()>;
}

/// A tool for scraping UNSW course and class data.
#[derive(FromArgs)]
struct Cli {
    #[argh(subcommand)]
    command: Command,

    /// enable debug logging
    #[argh(switch, short = 'v')]
    verbose: bool,
}

#[derive(FromArgs)]
#[argh(subcommand)]
#[enum_dispatch(Exec)]
enum Command {
    Scrape(Scrape),
    BatchInsert(BatchInsert),
    ScrapeAndBatchInsert(ScrapeAndBatchInsert),
}

/// Perform scraping. Creates a JSON file to store the data.
#[derive(FromArgs)]
#[argh(subcommand, name = "scrape")]
struct Scrape {}

impl Exec for Scrape {
    async fn exec(&self) -> anyhow::Result<()> {
        handle_scrape_write_to_file().await
    }
}

/// Perform batch insert on JSON files created by `scrape`.
#[derive(FromArgs)]
#[argh(subcommand, name = "batch_insert")]
struct BatchInsert {}

impl Exec for BatchInsert {
    async fn exec(&self) -> anyhow::Result<()> {
        handle_batch_insert().await
    }
}

/// Perform scraping and batch insert. Does not create a JSON file to store the data.
#[derive(FromArgs)]
#[argh(subcommand, name = "scrape_n_batch_insert")]
struct ScrapeAndBatchInsert {}

impl Exec for ScrapeAndBatchInsert {
    async fn exec(&self) -> anyhow::Result<()> {
        handle_scrape_n_batch_insert().await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    let cli: Cli = argh::from_env();

    let lvl = if cli.verbose {
        // TODO: we don't currently have any Debug logs, but useful for when we do, or we can config differently.
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    env_logger::Builder::new()
        .filter_level(lvl)
        // Only show error logs from html5ever dependency, since it logs many unnecessary warnings.
        .filter_module("html5ever", LevelFilter::Error)
        .init();

    cli.command.exec().await?;

    Ok(())
}
