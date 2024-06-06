use reqwest::ClientBuilder;

pub trait Scraper {
    fn scrape(&mut self) -> impl std::future::Future<Output = Result<(), Box<dyn std::error::Error>>> + Send;
}

pub async fn fetch_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    Ok(body)
}

