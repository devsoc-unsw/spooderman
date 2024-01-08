use spooderman::{Scraper, SubjectAreaScraper};

#[tokio::main]
async fn main() {
    let mut scraper = SubjectAreaScraper::new()
        .set_url("https://timetable.unsw.edu.au/2024/subjectSearch.html".to_string());
    match scraper.run_scraper_on_url().await {
        Ok(_) => {
            println!("Scraping successful!\n");
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
