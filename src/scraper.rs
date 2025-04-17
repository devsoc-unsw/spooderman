use reqwest::ClientBuilder;

pub async fn fetch_url(url: &str) -> anyhow::Result<String> {
    let client = ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()?;
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    Ok(body)
}
