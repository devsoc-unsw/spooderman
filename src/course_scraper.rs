use derive_new::new;
use rayon::prelude::*;
use scraper::Selector;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::{ScrapingContext, Year, text_manipulators::extract_text};

#[derive(Debug, Serialize)]
pub struct Course {
    pub course_id: String,
    pub course_code: String,
    pub year: Year,
    pub course_name: String,
    pub uoc: i32,
    pub faculty: Option<String>,
    pub school: Option<String>,
    pub career: String,
    // Sorted ascendingly.
    pub modes: Vec<String>, // For Notangles.
    pub campus: Option<String>,
    pub terms: Vec<String>,
    pub classes: Vec<Class>,
}

#[derive(Debug, Serialize)]
pub struct Class {
    pub course_id: String,
    pub career: String,
    pub class_id: String,
    pub class_nr: String,
    pub section: String,
    pub term: String,
    pub year: Year,
    pub activity: String,
    pub status: String,
    pub course_enrolment: String,
    pub offering_period: String,
    pub meeting_dates: String,
    pub census_date: String,
    pub consent: String,
    pub mode: String,
    pub times: Option<Vec<Time>>,
    pub class_notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Time {
    pub time_id: String,
    pub career: String,
    pub location: String,
    pub day: String,
    pub time: String,
    pub weeks: String,
    pub instructor: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PartialTime {
    pub location: String,
    pub day: String,
    pub time: String,
    pub weeks: String,
    pub instructor: Option<String>,
}

#[derive(Debug, new)]
pub struct PartialCourse {
    pub course_code: String,
    pub course_name: String,
    pub career: String,
    pub uoc: i32,
    pub url: String,
    pub year: Year,
}

impl PartialCourse {
    pub async fn complete(self, ctx: &ScrapingContext) -> anyhow::Result<Course> {
        let html = ctx.request_client.fetch_url_body(&self.url, ctx).await?;
        let course_code = self.course_code.clone();

        let cpu_bound = move || -> anyhow::Result<Course> {
            let document = scraper::Html::parse_document(&html);

            // NOTE: We can't return the error message from `Selector::parse`
            // because it is not Send and, therefore, not sendable across threads.
            let error_msg = format!("failed to parse {}", self.url);

            // Selectors
            let form_bodies = Selector::parse("td.formBody td.formBody")
                .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
            let term_selector = Selector::parse("table table:nth-of-type(3)")
                .map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
            let table_selector =
                Selector::parse("table table").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
            let label_selector =
                Selector::parse("td.label").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
            let data_selector =
                Selector::parse("td.data").map_err(|_| anyhow::anyhow!(error_msg.clone()))?;
            let information_body = document.select(&form_bodies);

            let career = self.career;
            let mut faculty = None;
            let mut school = None;
            let mut campus = None;

            let mut skip_this_info_box = false;
            let mut terms: Vec<String> = vec![];
            let mut class_activity_information: Vec<Vec<String>> = vec![];
            for info_box in information_body {
                if let Some(label_info) = info_box.select(&label_selector).next() {
                    // Check if it is a form body with course information
                    if extract_text(label_info).trim() == "Faculty" {
                        let labels: Vec<_> = info_box
                            .select(&label_selector)
                            .map(|el| extract_text(el).trim().replace("\u{a0}", ""))
                            .collect();

                        let data: Vec<_> = info_box
                            .select(&data_selector)
                            .map(|el| extract_text(el).trim().replace("\u{a0}", ""))
                            .collect();
                        for (label, data) in labels.iter().zip(data.into_iter()) {
                            match label.trim().to_lowercase().as_str() {
                                "faculty" => faculty = Some(data),
                                "school" => school = Some(data),
                                "campus" => campus = Some(data),
                                "career" => {
                                    if career != data {
                                        skip_this_info_box = true;
                                        break;
                                    } else {
                                        skip_this_info_box = false;
                                    }
                                }
                                _ => {}
                            }
                        }
                        if skip_this_info_box {
                            continue;
                        }
                        if let Some(terms_info_table) = info_box.select(&term_selector).next() {
                            for terms_table in terms_info_table.select(&table_selector) {
                                let curr_terms_row = terms_table
                                    .text()
                                    .map(|e| e.trim().to_string())
                                    .filter(|s| !s.is_empty())
                                    .collect::<Vec<_>>();
                                if !curr_terms_row.is_empty() {
                                    terms.extend(curr_terms_row);
                                }
                            }
                        }
                    } else if extract_text(label_info).trim() == "Class Nbr" && !skip_this_info_box
                    {
                        // Extract class.
                        let info_map = info_box
                            .select(
                                &Selector::parse("td.label, td.data")
                                    .map_err(|_| anyhow::anyhow!(error_msg.clone()))?,
                            )
                            .map(|cell| {
                                cell.text()
                                    .collect::<String>()
                                    .trim()
                                    .replace("\u{a0}", "")
                                    .to_string()
                            })
                            .collect::<Vec<_>>();
                        if !info_map.is_empty() {
                            class_activity_information.push(info_map);
                        }
                    }
                }
            }

            // The reason we aren't including the term in the course id is that the handbook only
            // contains one page per course per year, which contains data for the course that year.
            // We use the same format to reduce duplicated data.
            let course_id = format!("{}-{}-{}", self.course_code, career, self.year);

            let course_code = self.course_code;
            let course_name = self.course_name;
            let uoc = self.uoc;

            let classes: Vec<Class> = class_activity_information
                .into_par_iter()
                .map(|class_data| {
                    parse_class_info(class_data, &course_id, &course_code, &career, self.year)
                })
                .collect::<anyhow::Result<_>>()?;

            let unique_modes: HashSet<&String> = classes.iter().map(|class| &class.mode).collect();
            let mut modes: Vec<String> = unique_modes.iter().map(|mode| mode.to_string()).collect();
            // Guarantee unique order by sorting, which Hashset doesn't.
            modes.sort();

            Ok(Course {
                course_id,
                course_code,
                year: self.year,
                course_name,
                uoc,
                faculty,
                school,
                campus,
                career,
                modes,
                terms,
                classes,
            })
        };
        let course = tokio::task::spawn_blocking(cpu_bound).await?;
        log::info!("Finished scraping course {}", course_code);
        course
    }
}

fn parse_class_info(
    class_data: Vec<String>,
    course_id: &str,
    course_code: &str,
    course_career: &str,
    course_year: Year,
) -> anyhow::Result<Class> {
    let mut map: HashMap<&str, &str> = HashMap::new();
    let mut i = 0;
    let mut partial_times_parsed = Vec::new();

    while i < class_data.len() {
        let key = &class_data[i];
        if key == "Meeting Information" {
            let mut j = i + 1;
            while j < class_data.len() && class_data[j] != "Class Notes" {
                j += 1;
            }
            partial_times_parsed = parse_meeting_info(&class_data[i + 1..j])?;
            i = j + 1;
            continue;
        }

        let value = if i + 1 < class_data.len() {
            &class_data[i + 1]
        } else {
            ""
        };
        map.insert(key, value);
        i += 2;
    }

    let missing_field_error = |missing_field_name: &str| {
        anyhow::anyhow!(format!(
            "{} field for course {} was missing while parsing a class",
            missing_field_name, course_code
        ))
    };
    let get_expected_field = |field_name: &str| {
        map.get(field_name)
            .ok_or_else(|| missing_field_error(field_name))
    };

    let offering_period_str = get_expected_field("Offering Period")?;
    let mut split_offering_period_str = offering_period_str.split(" - ");

    let section = get_expected_field("Section")?;

    let date = split_offering_period_str
        .next()
        .ok_or_else(|| missing_field_error("date"))?;

    let class_year: Year = date
        .split("/")
        .nth(2)
        .ok_or_else(|| missing_field_error("year"))?
        .parse()?;
    assert!(class_year == course_year || class_year == (course_year - 1));
    // If the course is in the summer term, the offering period might start in the previous year, so we use the course year.
    let year = course_year;

    let class_nr = get_expected_field("Class Nbr")?;
    let term = get_expected_field("Teaching Period")?
        .split(" - ")
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!(format!(
                "failed to parse term from teaching period for course {}",
                course_code
            ))
        })?;

    let class_id = format!(
        "{}-{}-{}-{}-{}",
        course_code, course_career, year, term, class_nr
    );
    let activity = get_expected_field("Activity")?;
    let status = get_expected_field("Status")?;
    let course_enrolment = get_expected_field("Enrols/Capacity")?.replace("*", "");
    let offering_period = get_expected_field("Offering Period")?;
    let meeting_dates = get_expected_field("Meeting Dates")?;
    let census_date = get_expected_field("Census Date")?;
    let mode = get_expected_field("Mode of Delivery")?;
    let consent = get_expected_field("Consent")?;
    let times = if !partial_times_parsed.is_empty() {
        Some(
            partial_times_parsed
                .into_iter()
                .map(|partial_time| {
                    partial_time.complete(course_code, course_career, year, term, class_nr)
                })
                .collect(),
        )
    } else {
        None
    };
    let class_notes = map
        .get("Class Notes")
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    Ok(Class {
        course_id: course_id.to_string(),
        class_id,
        class_nr: class_nr.to_string(),
        section: section.to_string(),
        term: term.to_string(),
        year,
        activity: activity.to_string(),
        status: status.to_string(),
        course_enrolment,
        offering_period: offering_period.to_string(),
        meeting_dates: meeting_dates.to_string(),
        census_date: census_date.to_string(),
        mode: mode.to_string(),
        consent: consent.to_string(),
        career: course_career.to_string(),
        times,
        class_notes,
    })
}

fn parse_meeting_info(vec: &[String]) -> anyhow::Result<Vec<PartialTime>> {
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut meetings = Vec::new();
    let mut iter: Box<dyn Iterator<Item = &String>> = Box::new(vec.iter());

    while let Some(day) = iter.next() {
        if days.contains(&day.as_str()) {
            let (Some(time), Some(location), Some(weeks)) = (iter.next(), iter.next(), iter.next())
            else {
                break;
            };

            // Optional instructor parsing
            let mut instructor: Option<String> = None;
            if let Some(some_instructor) = iter.next() {
                if !days.contains(&some_instructor.as_str()) {
                    instructor = Some(some_instructor.clone());
                } else {
                    iter = Box::new(std::iter::once(some_instructor).chain(iter));
                }
            }

            let timeslot = PartialTime {
                day: day.to_string(),
                time: time.to_string(),
                location: location.to_string(),
                weeks: weeks.to_string(),
                instructor,
            };

            meetings.push(timeslot);
        }
    }

    Ok(meetings)
}

impl PartialTime {
    fn complete(
        self,
        course_code: &str,
        course_career: &str,
        year: Year,
        term: &str,
        class_nr: &str,
    ) -> Time {
        let time_id = format!(
            "{}_{}_{}_{}_{}_{}_{}_{}_{}",
            course_code,
            course_career,
            year,
            term,
            class_nr,
            self.location,
            self.day,
            self.time,
            self.weeks
        );
        Time {
            time_id,
            career: course_career.to_string(),
            location: self.location,
            day: self.day,
            time: self.time,
            weeks: self.weeks,
            instructor: self.instructor,
        }
    }
}
