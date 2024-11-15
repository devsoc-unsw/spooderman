use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::vec;

#[derive(Serialize, Deserialize)]
struct Metadata {
    table_name: String,
    columns: Vec<String>,
    sql_up: String,
    sql_down: String,
    write_mode: Option<String>,
    sql_before: Option<String>,
    sql_after: Option<String>,
    dryrun: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct BatchInsertRequest {
    metadata: Metadata,
    payload: Vec<serde_json::Value>,
}

fn read_json_file(file_path: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let data: Vec<Value> = serde_json::from_str(&contents)?;
    Ok(data)
}

fn read_sql_file(file_path: &str) -> std::io::Result<String> {
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
pub struct ReadFromFile;
pub struct ReadFromMemory {
    pub courses_vec: Vec<Value>,
    pub classes_vec: Vec<Value>,
    pub times_vec: Vec<Value>,
}

pub trait HasuragresData {
    fn get_courses(&self) -> Vec<Value>;
    fn get_classes(&self) -> Vec<Value>;
    fn get_times(&self) -> Vec<Value>;
}
impl HasuragresData for ReadFromFile {
    fn get_courses(&self) -> Vec<Value> {
        read_json_file("courses.json").expect("Could not read courses.json file!")
    }
    fn get_classes(&self) -> Vec<Value> {
        read_json_file("classes.json").expect("Could not read classes.json file!")
    }
    fn get_times(&self) -> Vec<Value> {
        read_json_file("times.json").expect("Could not read times.json file!")
    }
}

impl HasuragresData for ReadFromMemory {
    fn get_courses(&self) -> Vec<Value> {
        self.courses_vec.clone()
    }
    fn get_classes(&self) -> Vec<Value> {
        self.classes_vec.clone()
    }
    fn get_times(&self) -> Vec<Value> {
        self.times_vec.clone()
    }
}

pub async fn send_batch_data(hdata: &impl HasuragresData) -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    let hasuragres_url = env::var("HASURAGRES_URL")?;
    let api_key = env::var("HASURAGRES_API_KEY")?;
    let client = Client::new();
    println!("{:?} {:?}", hasuragres_url, api_key);
    println!("Starting to insert into Hasuragres!");
    let requests = vec![
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "courses".to_string(),
                columns: vec![
                    "course_code".to_string(),
                    "course_name".to_string(),
                    "uoc".to_string(),
                    "faculty".to_string(),
                    "school".to_string(),
                    "campus".to_string(),
                    "career".to_string(),
                    "terms".to_string(),
                    "modes".to_string(),
                ],
                sql_up: read_sql_file("sql/Courses/up.sql")?,
                sql_down: read_sql_file("sql/Courses/down.sql")?,
                write_mode: Some("overwrite".to_string()),
                sql_before: None,
                sql_after: None,
                dryrun: Some(true),
            },
            payload: hdata.get_courses(),
        },
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "classes".to_string(),
                columns: vec![
                    "class_id".to_string(),
                    "career".to_string(),
                    "course_id".to_string(),
                    "section".to_string(),
                    "term".to_string(),
                    "activity".to_string(),
                    "year".to_string(),
                    "status".to_string(),
                    "course_enrolment".to_string(),
                    "offering_period".to_string(),
                    "meeting_dates".to_string(),
                    "census_date".to_string(),
                    "consent".to_string(),
                    "mode".to_string(),
                    "class_notes".to_string(),
                ],
                sql_up: read_sql_file("sql/Classes/up.sql")?,
                sql_down: read_sql_file("sql/Classes/down.sql")?,
                write_mode: Some("overwrite".to_string()),
                sql_before: None,
                sql_after: None,
                dryrun: Some(true),
            },
            payload: hdata.get_classes(),
        },
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "times".to_string(),
                columns: vec![
                    "id".to_string(),
                    "class_id".to_string(),
                    "career".to_string(),
                    "day".to_string(),
                    "instructor".to_string(),
                    "location".to_string(),
                    "time".to_string(),
                    "weeks".to_string(),
                ],
                sql_up: read_sql_file("sql/Times/up.sql")?,
                sql_down: read_sql_file("sql/Times/down.sql")?,
                write_mode: Some("overwrite".to_string()),
                sql_before: None,
                sql_after: None,
                dryrun: Some(true),
            },
            payload: hdata.get_times(),
        },
    ];

    let response = client
        .post(format!("{}/batch_insert", hasuragres_url))
        .header("X-API-Key", api_key)
        .json(&requests)
        .send()
        .await;

    match response {
        Ok(res) => {
            if res.status().as_u16() == 400 {
                let error_body: Result<Value, reqwest::Error> = res.json().await;

                match error_body {
                    Ok(json) => {
                        println!("Error occurred: {:?}", json);
                        if let Some(error_message) = json.get("error") {
                            println!("Error message: {}", error_message);
                        }
                    }
                    Err(err) => {
                        eprintln!("Failed to parse error body: {:?}", err);
                    }
                }
            } else {
                let data: Result<Value, reqwest::Error> = res.json().await;
                match data {
                    Ok(_) => println!("Successfully inserted into Hasuragres"),
                    Err(err) => eprintln!("Failed to parse response body: {:?}", err),
                }
            }
        }
        Err(e) => eprintln!("Failed to insert batch data: {:?}", e),
    }
    Ok(())
}
