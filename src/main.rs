
mod scraper;
fn main() {
    let response = reqwest::blocking::get("https://scrapeme.live/shop/");
    let html_content = response.unwrap().text().unwrap();
    println!("{:?}", html_content);
}
