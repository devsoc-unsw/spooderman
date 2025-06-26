use crate::{
    Course, ScrapingContext,
    subject_area_scraper::SubjectArea,
    text_manipulators::{extract_text, get_html_link_to_page},
};
use derive_new::new;
use scraper::Selector;
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct SchoolArea {
    pub url: String,
    pub pages: Vec<SchoolAreaPage>,
}

impl SchoolArea {
    pub async fn scrape(url: String, ctx: &Arc<ScrapingContext>) -> anyhow::Result<Self> {
        log::info!("Started scraping School Area for: {}", url);

        let html = ctx.request_client.fetch_url_body(&url).await?;

        // We use a channel so we can start completing a partial page
        // immediately once it's scraped, so we don't have to wait until all
        // partial pages have been scraped.
        let (tx, mut rx) = mpsc::unbounded_channel();

        // TODO: remove all unwraps in producer and return Result instead (possible, i've just been lazy)
        let producer = async || {
            let ctx = Arc::clone(ctx);
            let url = url.clone();
            let cpu_bound = move || {
                let document = scraper::Html::parse_document(&html);

                let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight").unwrap();
                let code_selector = Selector::parse("td.data").unwrap();
                let name_selector = Selector::parse("td.data a").unwrap();
                let link_selector = Selector::parse("td.data a").unwrap();
                let school_selector = Selector::parse("td.data:nth-child(3)").unwrap();

                for row_node in document.select(&row_selector) {
                    // Extract data from each row
                    let course_code = extract_text(row_node.select(&code_selector).next().unwrap());
                    let course_name = extract_text(row_node.select(&name_selector).next().unwrap());
                    let year_to_scrape =
                        ctx.timetable_url_year_extractor.extract_year(&url).unwrap();
                    let url_to_scrape_further = get_html_link_to_page(
                        year_to_scrape,
                        row_node
                            .select(&link_selector)
                            .next()
                            .map_or("", |node| node.value().attr("href").unwrap_or("")),
                        &ctx,
                    );
                    let school = extract_text(row_node.select(&school_selector).next().unwrap());
                    let partial_page = PartialSchoolAreaPage::new(
                        course_code,
                        course_name,
                        school,
                        url_to_scrape_further,
                    );
                    tx.send(partial_page).unwrap();
                }
            };
            tokio::task::spawn_blocking(cpu_bound).await
        };

        let mut consumer = async move || {
            let mut tasks = tokio::task::JoinSet::new();

            // Spawn partial-page-completion tasks as soon as we receive partial pages.
            while let Some(partial_page) = rx.recv().await {
                let ctx = Arc::clone(ctx);
                tasks.spawn(async move { partial_page.complete(&ctx).await });
            }

            // Wait for all partial-page-completion tasks to complete.
            tasks.join_all().await
        };

        // Wait on producer and consumer.
        let (_, result_pages) = tokio::join!(producer(), consumer());
        let pages: Vec<SchoolAreaPage> = result_pages.into_iter().collect::<anyhow::Result<_>>()?;

        Ok(Self { url, pages })
    }

    pub fn get_all_courses(self) -> impl Iterator<Item = Course> {
        self.pages
            .into_iter()
            .flat_map(|school_area_page| school_area_page.subject_area.courses)
    }
}

#[derive(Debug, new)]
pub struct SchoolAreaPage {
    pub course_code: String,
    pub course_name: String,
    pub school: String,
    pub subject_area: SubjectArea,
}

#[derive(Debug, new)]
struct PartialSchoolAreaPage {
    course_code: String,
    course_name: String,
    school: String,
    subject_area_url: String,
}

impl PartialSchoolAreaPage {
    async fn complete(self, ctx: &Arc<ScrapingContext>) -> anyhow::Result<SchoolAreaPage> {
        let subject_area = SubjectArea::scrape(self.subject_area_url, ctx).await?;
        Ok(SchoolAreaPage::new(
            self.course_code,
            self.course_name,
            self.school,
            subject_area,
        ))
    }
}
