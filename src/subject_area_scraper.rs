use std::{collections::HashSet, sync::Arc};

use anyhow::Context;
use scraper::{ElementRef, Selector};
use tokio::sync::mpsc;

use crate::{
    Course, ScrapingContext,
    course_scraper::PartialCourse,
    text_manipulators::{extract_text, get_html_link_to_page},
};

#[derive(Debug)]
pub struct SubjectArea {
    pub courses: Vec<Course>,
}

impl SubjectArea {
    pub async fn scrape(url: String, ctx: &Arc<ScrapingContext>) -> anyhow::Result<Self> {
        log::info!("Started scraping Subject Area for: {}", url);

        let html = ctx.request_client.fetch_url_body(&url).await?;

        // We use a channel so we can start completing a partial course
        // immediately once it's scraped, so we don't have to wait until all
        // partial courses have been scraped.
        let (tx, mut rx) = mpsc::unbounded_channel();

        let producer = async move || -> anyhow::Result<()> {
            let ctx = Arc::clone(ctx);

            let cpu_bound = move || -> anyhow::Result<()> {
                let document = scraper::Html::parse_document(&html);

                // NOTE: We can't return the error message from `Selector::parse`
                // because it is not Send and, therefore, not sendable across threads.
                let error_msg = format!("failed to parse {}", url);

                let career_selector = Selector::parse("td.classSearchMinorHeading")
                    .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let row_selector = Selector::parse("tr.rowLowlight, tr.rowHighlight")
                    .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let code_selector =
                    Selector::parse("td.data").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let name_selector =
                    Selector::parse("td.data a").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let link_selector =
                    Selector::parse("td.data a").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let uoc_selector = Selector::parse("td.data:nth-child(3)")
                    .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                let mut visited_courses = HashSet::<String>::new();

                for career_elem_ref in document.select(&career_selector) {
                    let career = extract_text(career_elem_ref);
                    if career.is_empty() {
                        continue;
                    };
                    for row_node in ElementRef::wrap(
                        career_elem_ref
                            .parent()
                            .ok_or_else(|| {
                                anyhow::anyhow!(format!(
                                    "{}: {}",
                                    &error_msg, "Expected career to be inside td element"
                                ))
                            })?
                            .next_sibling()
                            .ok_or_else(|| {
                                anyhow::anyhow!(format!(
                                    "{}: {}",
                                    &error_msg,
                                    "Expected career classes td element to come after careers"
                                ))
                            })?
                            .next_sibling()
                            .ok_or_else(|| {
                                anyhow::anyhow!(format!(
                                    "{}: {}",
                                    &error_msg,
                                    "Expected career classes td element to come after careers"
                                ))
                            })?,
                    )
                    .ok_or_else(|| anyhow::anyhow!(error_msg.clone()))?
                    .select(&row_selector)
                    {
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
                                .nth(1)
                                .ok_or_else(|| anyhow::anyhow!(error_msg.clone()))?,
                        );
                        let name_hash = format!("{}{}", &course_code, &career);
                        if visited_courses.contains(&name_hash) {
                            continue;
                        }
                        visited_courses.insert(name_hash);
                        let year_to_scrape = ctx
                            .timetable_url_year_extractor
                            .extract_year(&url)
                            .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
                        let url_to_scrape_further = get_html_link_to_page(
                            year_to_scrape,
                            row_node
                                .select(&link_selector)
                                .next()
                                .map_or("", |node| node.value().attr("href").unwrap_or("")),
                            &ctx,
                        );
                        let uoc = extract_text(
                            row_node
                                .select(&uoc_selector)
                                .next()
                                .ok_or_else(|| anyhow::anyhow!(error_msg.clone()))?,
                        )
                        .parse()
                        .context("Could not parse UOC!")?;

                        let course_scraper = PartialCourse::new(
                            course_code,
                            course_name,
                            career.trim().to_string(),
                            uoc,
                            url_to_scrape_further,
                        );
                        tx.send(course_scraper)?;
                    }
                }
                Ok(())
            };

            // NOTE: tokio is, by default, not designed for long running cpu bound tasks to be spawned, since it's designed for doing blocking IO asyncronously. current bottleneck: we're doing heavy cpu bound work on e.g. 42 OS threads (example execution measured once), which creates some scheduling overhead -> either limit to num cpus OS threads, since we don't use spawn_blocking for blocking io anyways, or look for different tokio API.
            tokio::task::spawn_blocking(cpu_bound).await??;
            Ok(())
        };

        let mut consumer = async move || -> anyhow::Result<Vec<Course>> {
            let mut tasks = tokio::task::JoinSet::new();

            // Spawn partial-page-completion tasks as soon as we receive partial pages.
            while let Some(partial_course) = rx.recv().await {
                let ctx = Arc::clone(ctx);
                tasks.spawn(async move { partial_course.complete(&ctx).await });
            }

            // Wait for all partial-course-completion tasks to complete.
            // If any of the scrapes returned an error, return the first encountered error.
            let courses: Vec<Course> = tasks
                .join_all()
                .await
                .into_iter()
                .collect::<anyhow::Result<_>>()?;
            Ok(courses)
        };

        // Wait on producer and consumer.
        let ((), courses) = tokio::try_join!(producer(), consumer())?;

        Ok(Self { courses })
    }
}
