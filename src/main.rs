// use spooderman::{Scraper, SubjectAreaScraper};
use dotenv::dotenv;
use regex::Regex;
use chrono::{Datelike, Utc};

extern crate log;
extern crate env_logger;

use log::LevelFilter;

use log::{info, warn, error};


fn mutate_string_to_include_curr_year(curr_base_url: &mut String) -> String { 
    let pattern = Regex::new("year").unwrap();
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
            mutate_string_to_include_curr_year(&mut url.to_string());
        
            // let mut scraper = SubjectAreaScraper::new().set_url(base_api_url.to_string());
            
            // match scraper.run_scraper_on_url().await {
            //     Ok(_) => {
            //         println!("Scraping successful!\n");
            //     }
            //     Err(e) => eprintln!("Error: {}", e),
            // }
        }
        Err(e) => {
            warn!("Timetable URL has NOT been parsed properly from env file and error report: {e}");
        }
    };
    
    // let mut scraper =  SubjectAreaScraper::new()
    //     .set_url(base_api_url.to_string());
    // 
}
