use spooderman::Scraper;


#[tokio::main]
async fn main() {
    let mut scraper = Scraper::new()
        .set_url("https://scrapeme.live/shop/".to_string());
    
    match scraper.scrape_website().await {
        Ok(_res) => {println!("Scraping successful!\n");
    },
        Err(e) => eprintln!("Error: {}", e),
    }
}
