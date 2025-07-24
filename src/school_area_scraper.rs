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

        let producer = async || -> anyhow::Result<()> {
            let ctx = Arc::clone(ctx);
            let url = url.clone();

            let cpu_bound = move || -> anyhow::Result<()> {
                let document = scraper::Html::parse_document(&html);

                // NOTE: We can't return the error message from `Selector::parse`
                // because it is not Send and, therefore, not sendable across threads.
                let error_msg = format!("failed to parse {}", url);

                let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight")
                    .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let code_selector =
                    Selector::parse("td.data").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let name_selector =
                    Selector::parse("td.data a").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let link_selector =
                    Selector::parse("td.data a").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let school_selector = Selector::parse("td.data:nth-child(3)")
                    .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;

                for row_node in document.select(&row_selector) {
                    // Extract data from each row
                    let course_code = extract_text(
                        row_node
                            .select(&code_selector)
                            .next()
                            .ok_or_else(|| anyhow::anyhow!(error_msg.clone()))?,
                    );
                    let course_name = extract_text(
                        row_node
                            .select(&name_selector)
                            .next()
                            .ok_or_else(|| anyhow::anyhow!(error_msg.clone()))?,
                    );

                    let year_to_scrape = ctx.timetable_url_year_extractor.extract_year(&url)?;
                    let url_to_scrape_further = get_html_link_to_page(
                        year_to_scrape,
                        row_node
                            .select(&link_selector)
                            .next()
                            .map_or("", |node| node.value().attr("href").unwrap_or("")),
                        &ctx,
                    );
                    let school = extract_text(
                        row_node
                            .select(&school_selector)
                            .next()
                            .ok_or_else(|| anyhow::anyhow!(error_msg.clone()))?,
                    );
                    let partial_page = PartialSchoolAreaPage::new(
                        course_code,
                        course_name,
                        school,
                        url_to_scrape_further,
                    );
                    tx.send(partial_page)?;
                }
                Ok(())
            };
            tokio::task::spawn_blocking(cpu_bound).await??;
            Ok(())
        };

        let mut consumer = async move || -> anyhow::Result<Vec<SchoolAreaPage>> {
            let mut tasks = tokio::task::JoinSet::new();

            // Spawn partial-X-completion tasks as soon as we receive them.
            while let Some(partial_page) = rx.recv().await {
                let ctx = Arc::clone(ctx);
                tasks.spawn(async move { partial_page.complete(&ctx).await });
            }

            // Wait for all partial-X-completion tasks to complete.
            // If any of the tasks returns an error, return that error
            // immediately (without waiting for all other tasks to finish).
            let mut pages = Vec::new();
            while let Some(result) = tasks.join_next().await {
                let course = result??;
                pages.push(course);
            }

            Ok(pages)
        };

        // Wait on producer and consumer.
        let ((), pages) = tokio::try_join!(producer(), consumer())?;

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
    pub subject_code: String,
    pub subject_name: String,
    pub school: String,
    pub subject_area: SubjectArea,
}

#[derive(Debug, new)]
struct PartialSchoolAreaPage {
    subject_code: String,
    subject_name: String,
    school: String,
    subject_area_url: String,
}

impl PartialSchoolAreaPage {
    async fn complete(self, ctx: &Arc<ScrapingContext>) -> anyhow::Result<SchoolAreaPage> {
        let subject_area = SubjectArea::scrape(self.subject_area_url, ctx).await?;
        Ok(SchoolAreaPage::new(
            self.subject_code,
            self.subject_name,
            self.school,
            subject_area,
        ))
    }
}
