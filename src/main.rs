use spooderman::Scraper;


#[tokio::main]
async fn main() {
    let mut scraper = Scraper::new()
        .set_url("https://timetable.unsw.edu.au/2024/subjectSearch.html".to_string());
    
    match scraper.run_scraper().await {
        Ok(_res) => {println!("Scraping successful!\n");
    },
        Err(e) => eprintln!("Error: {}", e),
    }
}
