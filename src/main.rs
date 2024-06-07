use spooderman::{ClassScraper, SchoolAreaScraper, Scraper, SubjectAreaScraper};
use dotenv::dotenv;
use regex::Regex;
use chrono::{Datelike, Utc};
use std::collections::HashMap;

extern crate log;
extern crate env_logger;

use log::LevelFilter;

use log::{info, warn, error};

type CourseCode = String;
type FacultyCode = String;

pub fn mutate_string_to_include_curr_year(curr_base_url: &mut String) -> String { 
    let pattern = Regex::new("year").unwrap();
    // let course_map: HashMap<CourseCode, Course> = HashMap::new();

    pattern.replace(&curr_base_url, Utc::now().year().to_string()).to_string()
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    env_logger::Builder::new()
        .filter_level(LevelFilter::Info)
        .init();


    // The base API url needs the year to be replaced with "year".
    // ie. https://timetable.unsw.edu.au/year/subjectSearch.html
    match std::env::var("TIMETABLE_API_URL") {
        Ok(url) => {
            info!("Timetable URL has been parsed from environment file: {url}!");
            let url_to_scrape = mutate_string_to_include_curr_year(&mut url.to_string());
            
            let mut scraper = ClassScraper {
                subject_area_course_code: "COMP1511".to_string(),
                subject_area_course_name: "COMP1511".to_string(),
                uoc: 6,
                url: "https://timetable.unsw.edu.au/2024/ACCT5999.html".to_string(), 
                // url: "https://timetable.unsw.edu.au/2024/COMP1511.html".to_string(),
            };
            let _ = scraper.scrape().await;
            // let mut scraper = SchoolAreaScraper::new(url_to_scrape);
            
            // // let mut scraper = SubjectAreaScraper::new("https://timetable.unsw.edu.au/2024/COMPKENS.html".to_string());
            // match scraper.scrape().await {
            //     Ok(_) => info!("Scraping successful!\n"),
            //     Err(e) => error!("Error: {}", e),
            // } 
            // for school_area_page in &mut scraper.pages {
            //     let _ = school_area_page.subject_area_scraper.scrape().await;
            // }
            // println!("{:?}", scraper);
        }
        Err(e) => {
            warn!("Timetable URL has NOT been parsed properly from env file and error report: {e}");
        }
    };
    
}
