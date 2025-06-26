use rayon::prelude::*;
use scraper::Selector;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::{
    school_area_scraper::ScrapeError, scraper::fetch_url, text_manipulators::extract_text,
};

#[derive(Debug, Serialize)]
pub struct Course {
    pub course_id: String,
    pub course_code: String,
    pub course_name: String,
    pub uoc: i32,
    pub faculty: Option<String>,
    pub school: Option<String>,
    pub career: Option<String>,
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
    pub section: String,
    pub term: String,
    pub year: String,
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
    pub career: String,
    pub day: String,
    pub time: String,
    pub location: String,
    pub weeks: String,
    pub instructor: Option<String>,
}

#[derive(Debug)]
pub struct ClassScraper {
    pub course_code: String,
    pub course_name: String,
    pub career: String,
    pub uoc: i32,
    pub url: String,
}

impl ClassScraper {
    pub async fn scrape(&mut self) -> Result<Course, Box<ScrapeError>> {
        println!("Currently working on {:?}", self.course_code);
        let html = fetch_url(&self.url)
            .await
            .unwrap_or_else(|_| panic!("Something was wrong with the URL: {}", self.url));
        let document = scraper::Html::parse_document(&html);

        // Selectors
        let form_bodies = Selector::parse("td.formBody td.formBody").unwrap();
        let term_selector = Selector::parse("table table:nth-of-type(3)").unwrap();
        let table_selector = Selector::parse("table table").unwrap();
        let label_selector = Selector::parse("td.label").unwrap();
        let data_selector = Selector::parse("td.data").unwrap();
        let information_body = document.select(&form_bodies);

        let course_id = self.course_code.clone() + &self.career.clone();
        let course_code = self.course_code.clone();
        let course_name = self.course_name.clone();
        let uoc = self.uoc;
        let mut faculty = None;
        let mut school = None;
        let mut campus = None;
        let career = Some(self.career.clone());

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
                    for (label, data) in labels.iter().zip(data.iter()) {
                        match label.trim().to_lowercase().as_str() {
                            "faculty" => faculty = Some(data.clone()),
                            "school" => school = Some(data.clone()),
                            "campus" => campus = Some(data.clone()),
                            "career" => {
                                if career != Some(data.clone()) {
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
                } else if extract_text(label_info).trim() == "Class Nbr" && !skip_this_info_box {
                    // Extract class.
                    let info_map = info_box
                        .select(&Selector::parse("td.label, td.data").unwrap())
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

        let classes: Vec<Class> = class_activity_information
            .into_par_iter()
            .map(|class_data| {
                parse_class_info(
                    class_data,
                    self.course_code.clone() + &self.career.clone(),
                    self.career.clone(),
                )
            })
            .collect();

        let unique_modes: HashSet<&String> = classes.iter().map(|class| &class.mode).collect();
        let mut modes: Vec<String> = unique_modes.iter().map(|mode| mode.to_string()).collect();
        // Guarantee unique order by sorting, which Hashset doesn't.
        modes.sort();

        Ok(Course {
            course_id,
            course_code,
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
    }
}
fn parse_class_info(class_data: Vec<String>, course_id: String, career: String) -> Class {
    let mut map = HashMap::new();
    let mut i = 0;
    let mut times_parsed = Vec::<Time>::new();

    while i < class_data.len() {
        let key = class_data[i].clone();
        if key == "Meeting Information" {
            let mut j = i + 1;
            while j < class_data.len() && class_data[j] != "Class Notes" {
                j += 1;
            }
            times_parsed = parse_meeting_info(&class_data[i + 1..j], career.clone());
            i = j + 1;
            continue;
        }

        let value = if i + 1 < class_data.len() {
            class_data[i + 1].clone()
        } else {
            "".to_string()
        };
        map.insert(key, value);
        i += 2;
    }
    let offering_period_str = map
        .get("Offering Period")
        .unwrap_or(&"".to_string())
        .to_string();
    let mut split_offering_period_str = offering_period_str.split(" - ");
    let date = split_offering_period_str.next().unwrap();
    let year = date.split("/").nth(2).unwrap();
    Class {
        course_id: course_id.clone(),
        class_id: format!(
            "{}-{}-{}-{}",
            course_id,
            map.get("Class Nbr").unwrap_or(&String::new()),
            map.get("Teaching Period")
                .unwrap_or(&"".to_string())
                .split(" - ")
                .next()
                .expect("Could not split teaching periods properly!"),
            year,
        ),
        section: map.get("Section").unwrap_or(&"".to_string()).to_string(),
        term: map
            .get("Teaching Period")
            .unwrap_or(&"".to_string())
            .split(" - ")
            .next()
            .expect("Could not split teaching periods properly!")
            .to_string(),
        year: year.to_string(),
        activity: map.get("Activity").unwrap_or(&"".to_string()).to_string(),
        status: map.get("Status").unwrap_or(&"".to_string()).to_string(),
        course_enrolment: map
            .get("Enrols/Capacity")
            .unwrap_or(&"".to_string())
            .replace("*", "")
            .to_string(),
        offering_period: map
            .get("Offering Period")
            .unwrap_or(&"".to_string())
            .to_string(),
        meeting_dates: map
            .get("Meeting Dates")
            .unwrap_or(&"".to_string())
            .to_string(),
        census_date: map
            .get("Census Date")
            .unwrap_or(&"".to_string())
            .to_string(),
        mode: map
            .get("Mode of Delivery")
            .unwrap_or(&"".to_string())
            .to_string(),
        consent: map.get("Consent").unwrap_or(&"".to_string()).to_string(),
        career,
        times: if times_parsed.is_empty() {
            None
        } else {
            Some(times_parsed)
        },
        class_notes: map
            .get("Class Notes")
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
    }
}

fn parse_meeting_info(vec: &[String], career: String) -> Vec<Time> {
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut meetings = Vec::new();
    let mut iter: Box<dyn Iterator<Item = &String>> = Box::new(vec.iter());

    while let Some(day) = iter.next() {
        if days.contains(&day.as_str()) {
            let mut timeslot = get_blank_time_struct();
            timeslot.day = day.clone();

            // Safely unwrap time, location, and weeks
            if let (Some(time), Some(location), Some(weeks)) =
                (iter.next(), iter.next(), iter.next())
            {
                timeslot.time = time.clone();
                timeslot.location = location.clone();
                timeslot.weeks = weeks.clone();
            } else {
                break;
            }

            // Optional instructor parsing
            if let Some(instructor) = iter.next() {
                if !days.contains(&instructor.as_str()) {
                    timeslot.instructor = Some(instructor.clone());
                } else {
                    iter = Box::new(std::iter::once(instructor).chain(iter));
                }
            }
            timeslot.career = career.clone();
            meetings.push(timeslot);
        }
    }

    meetings
}

fn get_blank_time_struct() -> Time {
    Time {
        career: "".to_string(),
        day: "".to_string(),
        time: "".to_string(),
        location: "".to_string(),
        weeks: "".to_string(),
        instructor: None,
    }
}
