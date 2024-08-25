use std::collections::HashMap;

use scraper::Selector;

use crate::{scraper::fetch_url, text_manipulators::extract_text};

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
    pub course_id: String, // FK to subject_area_course_code
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
    pub async fn scrape(&mut self) -> Result<Course, Box<dyn std::error::Error>> {
        let html = fetch_url(&self.url).await?;

        let document = scraper::Html::parse_document(&html);

        let form_bodies = Selector::parse("td.formBody td.formBody").unwrap();
        let information_body = document.select(&form_bodies).skip(0).next().unwrap();
        let table_selector =
            Selector::parse("td.formBody > table:nth-of-type(1) > tbody > tr").unwrap();
        let label_selector = Selector::parse("td.label").unwrap();
        let data_selector = Selector::parse("td.data").unwrap();
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
        /*
         * This is for the banner information.
         */
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

        // Parse top header for term data
        let term_course_information_table =
            Selector::parse("td.formBody td.formBody table:nth-of-type(3) tbody tbody").unwrap();
        for row in document.select(&term_course_information_table) {
            course_info
                .terms
                .push(extract_text(row).trim().replace("\u{a0}", ""));
        }

        let term_course_information_table =
            Selector::parse("td.formBody td.formBody table").unwrap();
        let term_count = course_info.terms.len();
        // 3 is a magical pattern number that is consistent with the way the handbook is setup.
        // This skips stuff in the course information panel (3 being the tables that follow in the tables).
        // It is multiplied by the term count so we can skip the summary stuff and go to the class details straight away.
        // Then we get to the actual Class data.
        let skip_count = 3 + term_count + 3 * term_count;
        let mut class_activity_information = vec![];
        for row in document
            .select(&term_course_information_table)
            .skip(skip_count)
        {
            let cell_selector = Selector::parse("td.label, td.data").unwrap();
            let mut cells: Vec<_> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>().trim().replace("\u{a0}", ""))
                .flat_map(|line| {
                    line.split('\n')
                        .filter(|text| !text.is_empty())
                        .map(String::from)
                        .collect::<Vec<_>>()
                })
                .collect();
            cells.iter_mut().for_each(|s| *s = s.trim().to_string());
            let cell = cells
                .into_iter()
                .filter(|s| !(s.is_empty()))
                .collect::<Vec<_>>();
            if cell.len() > 0 && cell[0] == "Class Nbr" {
                class_activity_information.push(cell);
            }
        }

        course_info.classes = parse_class_info(
            class_activity_information,
            self.subject_area_course_code.clone(),
        );

        Ok(course_info)
    }
}

fn parse_class_info(data: Vec<Vec<String>>, course_id: String) -> Vec<Class> {
    let mut classes = Vec::new();
    for class_data in data {
        let mut map = HashMap::new();

        let mut i = 0;
        let mut times_parsed = Vec::<Time>::new();
        while i < class_data.len() {
            let key = class_data[i].clone();
            if key == "Meeting Information" {
                // println!("FOUND IT");
                let mut j = i + 1;
                if class_data[j] != "Class Notes" {
                    while class_data[j] != "Class Notes" && j < class_data.len() {
                        j += 1;
                    }
                    // [i, j) is what we need to parse.
                    times_parsed = parse_meeting_info(&class_data[i + 1..j]);
                    i = j + 1;
                    continue;
                }
            }

            let value = if i + 1 < class_data.len() {
                class_data[i + 1].clone()
            } else {
                "".to_string()
            };
            map.insert(key, value);
            i += 2;
        }

        let class_info = Class {
            course_id: course_id.clone(),
            class_id: course_id.clone() + "-" + map.get("Class Nbr").unwrap_or(&"".to_string()),
            section: map.get("Section").unwrap_or(&"".to_string()).to_string(),
            term: map
                .get("Teaching Period")
                .unwrap_or(&"".to_string())
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
        };

        classes.push(class_info);
    }

    classes
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

// fn parse_meeting_info(vec: &[String]) -> Vec<Time> {
//     let mut meetings = Vec::new();
//     let days = vec![
//         "Mon".to_string(),
//         "Tue".to_string(),
//         "Wed".to_string(),
//         "Thu".to_string(),
//         "Fri".to_string(),
//         "Sat".to_string(),
//         "Sun".to_string(),
//     ];

//     let mut curr_timeslot = get_blank_time_struct();
//     let mut i = 0;
//     while i < vec.len() {
//         if days.contains(&vec[i]) {
//             curr_timeslot = get_blank_time_struct();
//             curr_timeslot.day = vec[i].clone();
//             i += 1;
//             curr_timeslot.time = vec[i].clone();
//             i += 1;
//             curr_timeslot.location = vec[i].clone();
//             i += 1;
//             curr_timeslot.weeks = vec[i].clone();
//             i += 1;
//             if i >= vec.len() || days.contains(&vec[i]) {
//                 curr_timeslot.instructor = None;
//                 i -= 1; // So we can let the caller function deal with the indexing.
//             } else {
//                 curr_timeslot.instructor = Some(vec[i].clone());
//             }
//             meetings.push(curr_timeslot);
//         }
//         i += 1;
//     }
//     meetings
// }

fn parse_meeting_info(vec: &[String]) -> Vec<Time> {
    let days = vec!["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    let mut meetings = Vec::new();
    let mut iter: Box<dyn Iterator<Item = &String>> = Box::new(vec.iter());

    while let Some(day) = iter.next() {
        if days.contains(&day.as_str()) {
            let mut timeslot = get_blank_time_struct();
            timeslot.day = day.clone();

            // Unwrap the time, location, and weeks safely
            if let (Some(time), Some(location), Some(weeks)) =
                (iter.next(), iter.next(), iter.next())
            {
                timeslot.time = time.clone();
                timeslot.location = location.clone();
                timeslot.weeks = weeks.clone();
            } else {
                break; // Early exit if we don't have enough data
            }

            // Check for optional instructor
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
