use argh::FromArgs;
use chrono::Datelike;
use enum_dispatch::enum_dispatch;
use parse_display::FromStr;
use serde::Serialize;
use serde_json::{json, to_writer_pretty};
use spooderman::{
    Class, Course, SchoolArea, ScrapingContext, Time, Year, send_batch_data, sort_by_key_ref,
};
use spooderman::{ReadFromFile, ReadFromMemory};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::vec;

use log::LevelFilter;

async fn run_all_school_offered_courses_scraper_job(
    year: Year,
    ctx: &Arc<ScrapingContext>,
) -> anyhow::Result<SchoolArea> {
    let url_to_scrape = ctx.scraping_config.get_timetable_api_url_for_year(year);
    SchoolArea::scrape(url_to_scrape, ctx).await
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
    format!(
        "{}{}{}{}{}",
        &class.class_id, &time.day, &time.location, &time.time, &time.weeks
    )
}

fn convert_classes_times_to_json(courses: &[Course]) -> Vec<serde_json::Value> {
    let mut times_json = Vec::<serde_json::Value>::new();
    for course in courses.iter() {
        for class in course.classes.iter() {
            if let Some(times) = &class.times {
                for time in times.iter() {
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

#[derive(Debug, Serialize)]
struct Data {
    all_courses: Vec<Course>,
}

impl Data {
    async fn scrape(year_to_scrape: &YearToScrape) -> anyhow::Result<Data> {
        let ctx = Arc::new(ScrapingContext::new()?);
        // TODO: Batch the 2024 and 2025 years out since both too big to insert into hasura
        let year = year_to_scrape.into_year(&ctx).await?;
        log::info!("Starting scrape for year: {year}");
        let school_area = run_all_school_offered_courses_scraper_job(year, &ctx).await?;

        let mut all_courses = school_area.get_all_courses().collect::<Vec<_>>();
        sort_by_key_ref(&mut all_courses, |course| &course.course_id);

        Ok(Data { all_courses })
    }

    async fn write_to_single_json(&self, json_file_path: &str) -> anyhow::Result<()> {
        log::info!("Writing scraped data to {}!", json_file_path);
        let file = File::create(json_file_path)?;
        to_writer_pretty(file, &self)?;
        Ok(())
    }

    async fn write_to_files(&self) -> anyhow::Result<()> {
        log::info!("Writing scraped data to disk!");
        let json_classes = convert_classes_to_json(&self.all_courses);
        let json_courses = convert_courses_to_json(&self.all_courses);
        let json_times = convert_classes_times_to_json(&self.all_courses);

        let file_classes = File::create("classes.json")?;
        let file_courses = File::create("courses.json")?;
        let file_times = File::create("times.json")?;
        to_writer_pretty(file_classes, &json_classes)?;
        to_writer_pretty(file_courses, &json_courses)?;
        to_writer_pretty(file_times, &json_times)?;
        Ok(())
    }

    async fn handle_batch_insert(&self) -> anyhow::Result<()> {
        let json_classes = convert_classes_to_json(&self.all_courses);
        let json_courses = convert_courses_to_json(&self.all_courses);
        let json_times = convert_classes_times_to_json(&self.all_courses);
        let rfm = ReadFromMemory {
            courses_vec: json_courses,
            classes_vec: json_classes,
            times_vec: json_times,
        };
        send_batch_data(&rfm).await?;
        Ok(())
    }
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

    send_batch_data(&ReadFromFile).await?;
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

#[derive(Debug, FromStr)]
enum YearToScrape {
    #[display("latest-with-data")]
    LatestYearWithDataAvailable,

    #[display("{0}")]
    Year(Year),
}

fn get_current_year() -> Year {
    chrono::Utc::now().year()
}

async fn year_has_data(year: Year, ctx: &ScrapingContext) -> anyhow::Result<bool> {
    let year_url = ctx.scraping_config.get_timetable_api_url_for_year(year);
    let response = ctx.request_client.fetch_url_response(&year_url).await?;
    // UNSW servers will return a 404 if the data for a year isn't available.
    match response.status().as_u16() {
        200 => Ok(true),
        404 => Ok(false),
        other => Err(anyhow::anyhow!(
            "UNSW servers returned an unexpected status code '{}' for a GET request to '{}'",
            other,
            year_url
        )),
    }
}

impl YearToScrape {
    async fn into_year(&self, ctx: &ScrapingContext) -> anyhow::Result<Year> {
        match self {
            YearToScrape::Year(year) => Ok(*year),
            YearToScrape::LatestYearWithDataAvailable => {
                // try to find the latest year in the future, the the latest in the past.

                // How far we potentially look into the future and past.
                const MAX_FUTURE_YEARS: i32 = 20;
                const MAX_PAST_YEARS: i32 = 20;

                let curr_year = get_current_year();

                // go as far as possible into future.
                let mut latest_in_future = None;
                for year in curr_year..curr_year + MAX_FUTURE_YEARS {
                    if year_has_data(year, ctx).await? {
                        latest_in_future = Some(year);
                    } else {
                        break;
                    }
                }
                if let Some(year) = latest_in_future {
                    return Ok(year);
                }

                // go until first possible in the past.
                // we've already checked the current year.
                for year in (curr_year - MAX_PAST_YEARS..curr_year).rev() {
                    if year_has_data(year, ctx).await? {
                        return Ok(year);
                    }
                }

                Err(anyhow::anyhow!(
                    "no year (neither in the future nor in the past relative to current year) has data"
                ))
            }
        }
    }
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
struct Scrape {
    /// the year for which data should be scraped: `latest-with-data` (the latest year with data available), or a calendar year, e.g. `2025`
    #[argh(option, long = "year", short = 'y')]
    year_to_scrape: YearToScrape,

    /// write to a single JSON file instead
    #[argh(option, long = "to-file")]
    write_to_json_file: Option<String>,
}

impl Exec for Scrape {
    async fn exec(&self) -> anyhow::Result<()> {
        log::info!("Handling scrape...");
        let data = Data::scrape(&self.year_to_scrape).await?;
        match &self.write_to_json_file {
            Some(json_file_path) => data.write_to_single_json(&json_file_path).await?,
            None => data.write_to_files().await?,
        }
        Ok(())
    }
}

/// Perform batch insert on JSON files created by `scrape`.
#[derive(FromArgs)]
#[argh(subcommand, name = "batch_insert")]
struct BatchInsert {}

impl Exec for BatchInsert {
    async fn exec(&self) -> anyhow::Result<()> {
        log::info!("Handling batch insert...");
        handle_batch_insert().await?;
        Ok(())
    }
}

/// Perform scraping and batch insert. Does not create a JSON file to store the data.
#[derive(FromArgs)]
#[argh(subcommand, name = "scrape_n_batch_insert")]
struct ScrapeAndBatchInsert {
    /// the year for which data should be scraped: `latest-with-data` (the latest year with data available), or a calendar year, e.g. `2025`.
    #[argh(option, long = "year", short = 'y')]
    year_to_scrape: YearToScrape,
}

impl Exec for ScrapeAndBatchInsert {
    async fn exec(&self) -> anyhow::Result<()> {
        log::info!("Handling scrape and batch insert...");
        let data = Data::scrape(&self.year_to_scrape).await?;
        data.write_to_files().await?;
        data.handle_batch_insert().await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli: Cli = argh::from_env();

    let lvl = if cli.verbose {
        // NOTE: we don't currently have any Debug logs, but useful for when we do, or we can config differently.
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
