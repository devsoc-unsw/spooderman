use rayon::prelude::*;
use scraper::Selector;
use std::collections::HashMap;

use crate::{
    school_area_scraper::ScrapeError, scraper::fetch_url, text_manipulators::extract_text,
};

#[derive(Debug)]
pub struct Course {
    pub subject_area_course_code: String,
    pub subject_area_course_name: String,
    pub uoc: i32,
    pub faculty: Option<String>,
    pub school: Option<String>,
    pub career: Option<String>,
    pub campus: Option<String>,
    pub terms: Vec<String>,
    pub classes: Vec<Class>,
}

#[derive(Debug)]
pub struct Class {
    pub course_id: String,
    pub class_id: String,
    pub section: String,
    pub term: String,
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

#[derive(Debug)]
pub struct Time {
    pub day: String,
    pub time: String,
    pub location: String,
    pub weeks: String,
    pub instructor: Option<String>,
}

#[derive(Debug)]
pub struct ClassScraper {
    pub subject_area_course_code: String,
    pub subject_area_course_name: String,
    pub uoc: i32,
    pub url: String,
}

impl ClassScraper {
    pub async fn scrape(&mut self) -> Result<Course, Box<ScrapeError>> {
        println!("Currently working on {:?}", self.subject_area_course_code);
        let html = fetch_url(&self.url)
            .await
            .expect(&format!("Something was wrong with the URL: {}", self.url));
        let document = scraper::Html::parse_document(&html);

        // Selectors
        let form_bodies = Selector::parse("td.formBody td.formBody").unwrap();
        let table_selector =
            Selector::parse("td.formBody > table:nth-of-type(1) > tbody > tr").unwrap();
        let label_selector = Selector::parse("td.label").unwrap();
        let data_selector = Selector::parse("td.data").unwrap();
        let term_course_information_table =
            Selector::parse("td.formBody td.formBody table").unwrap();
        let information_body = document.select(&form_bodies).next().unwrap();
        let mut course_info = Course {
            subject_area_course_code: self.subject_area_course_code.clone(),
            subject_area_course_name: self.subject_area_course_name.clone(),
            uoc: self.uoc,
            faculty: None,
            school: None,
            campus: None,
            career: None,
            terms: vec![],
            classes: vec![],
        };

        // Extract banner information
        for row in information_body.select(&table_selector) {
            let labels: Vec<_> = row
                .select(&label_selector)
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .collect();
            let data: Vec<_> = row
                .select(&data_selector)
                .map(|el| el.text().collect::<Vec<_>>().join(""))
                .collect();

            for (label, data) in labels.iter().zip(data.iter()) {
                match label.trim().to_lowercase().as_str() {
                    "faculty" => course_info.faculty = Some(data.clone()),
                    "school" => course_info.school = Some(data.clone()),
                    "campus" => course_info.campus = Some(data.clone()),
                    "career" => course_info.career = Some(data.clone()),
                    _ => {}
                }
            }
        }

        // Parse terms
        let term_data_selector = Selector::parse(
            "td.formBody td.formBody table:nth-of-type(3) td.data td.data:nth-of-type(2)",
        )
        .unwrap();
        let term_data = document
            .select(&term_data_selector)
            .map(|row| extract_text(row).trim().replace("\u{a0}", ""))
            .collect::<Vec<_>>();

        course_info.terms = term_data.clone();

        // Skip header and course info, and go to class details
        let skip_count = 3 + term_data.len() + 3 * term_data.len();
        let class_activity_information: Vec<Vec<String>> = document
            .select(&term_course_information_table)
            .skip(skip_count)
            .map(|row| {
                row.select(&Selector::parse("td.label, td.data").unwrap())
                    .map(|cell| {
                        cell.text()
                            .collect::<String>()
                            .trim()
                            .replace("\u{a0}", "")
                            .to_string()
                    })
                    .collect()
            })
            .filter(|cells: &Vec<String>| !cells.is_empty() && cells[0] == "Class Nbr")
            .collect();

        course_info.classes = class_activity_information
            .into_par_iter()
            .map(|class_data| parse_class_info(class_data, self.subject_area_course_code.clone()))
            .collect();
        Ok(course_info)
    }
}

fn parse_class_info(class_data: Vec<String>, course_id: String) -> Class {
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
            times_parsed = parse_meeting_info(&class_data[i + 1..j]);
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

    Class {
        course_id: course_id.clone(),
        class_id: format!(
            "{}-{}",
            course_id,
            map.get("Class Nbr").unwrap_or(&String::new())
        ),
        section: map.get("Section").unwrap_or(&"".to_string()).to_string(),
        term: map
            .get("Teaching Period")
            .unwrap_or(&"".to_string())
            .to_string()
            .split(" - ")
            .next()
            .expect("Could not split teaching periods properly!")
            .to_string(),
        activity: map.get("Activity").unwrap_or(&"".to_string()).to_string(),
        status: map.get("Status").unwrap_or(&"".to_string()).to_string(),
        course_enrolment: map
            .get("Enrols/Capacity")
            .unwrap_or(&"".to_string())
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

fn parse_meeting_info(vec: &[String]) -> Vec<Time> {
    let days = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
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

            meetings.push(timeslot);
        }
    }

    meetings
}

fn get_blank_time_struct() -> Time {
    Time {
        day: "".to_string(),
        time: "".to_string(),
        location: "".to_string(),
        weeks: "".to_string(),
        instructor: None,
    }
}
