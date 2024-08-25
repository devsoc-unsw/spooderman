use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
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

pub async fn send_batch_data() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();
    let hasuragres_url = env::var("HASURAGRES_URL")?;
    let api_key = env::var("HASURAGRES_API_KEY")?;
    let client = Client::new();
    let requests = vec![
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "courses".to_string(),
                columns: vec![
                    "subject_area_course_code".to_string(),
                    "subject_area_course_name".to_string(),
                    "uoc".to_string(),
                    "faculty".to_string(),
                    "school".to_string(),
                    "campus".to_string(),
                    "career".to_string(),
                    "terms".to_string(),
                ],
                sql_up: read_sql_file("sql/Courses/up.sql")?,
                sql_down: read_sql_file("sql/Courses/down.sql")?,
                write_mode: Some("overwrite".to_string()),
                sql_before: None,
                sql_after: None,
                dryrun: Some(false),
            },
            payload: read_json_file("courses.json")?,
        },
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "classes".to_string(),
                columns: vec![
                    "class_id".to_string(),
                    "course_id".to_string(),
                    "section".to_string(),
                    "term".to_string(),
                    "activity".to_string(),
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
                dryrun: Some(false),
            },
            payload: read_json_file("classes.json")?,
        },
        BatchInsertRequest {
            metadata: Metadata {
                table_name: "times".to_string(),
                columns: vec![
                    "class_id".to_string(),
                    "course_id".to_string(),
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
                dryrun: Some(false),
            },
            payload: read_json_file("times.json")?,
        },
    ];

    let response = client
        .post(format!("{}/batch_insert", hasuragres_url))
        .header("X-API-Key", api_key)
        .json(&requests)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Batch data inserted successfully!");
    } else {
        eprintln!("Failed to insert batch data: {:?}", response.text().await?);
    }

    Ok(())
}
