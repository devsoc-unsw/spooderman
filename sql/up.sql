CREATE TYPE career_enum AS ENUM ('UG', 'PG', 'RESEARCH');
CREATE TYPE term_enum   AS ENUM ('T1', 'T2', 'T3', 'Summer');
CREATE TYPE status_enum AS ENUM ('Open', 'Closed');

-- Create Courses table with enum for career
CREATE TABLE Courses (
    "subject_area_course_code"        VARCHAR(8) PRIMARY KEY, --id
    "subject_area_course_name"        VARCHAR(50) NOT NULL,
    "uoc"                             INT NOT NULL,
    "faculty"                         VARCHAR(50),
    "school"                          VARCHAR(50),
    "campus"                          VARCHAR(50),
    "career"                          career_enum,
    "terms"                           VARCHAR(2)[],
);

CREATE TABLE Classes (
    "class_id"                        VARCHAR(255) PRIMARY KEY,
    "course_id"                       VARCHAR(8) NOT NULL,
    "section"                         VARCHAR(255) NOT NULL,
    "term"                            term_enum, 
    "activity"                        VARCHAR(255) NOT NULL,
    "status"                          status_enum,
    "course_enrolment"                VARCHAR(255) NOT NULL,
    "offering_period"                 VARCHAR(255) NOT NULL,
    "meeting_dates"                   VARCHAR(255) NOT NULL,
    "census_date"                     DATE NOT NULL,
    "consent"                         VARCHAR(255) NOT NULL,
    "mode"                            VARCHAR(255) NOT NULL,
    "class_notes"                     TEXT
    FOREIGN KEY ("course_id") REFERENCES Courses("subject_area_course_code") ON DELETE CASCADE
);

CREATE TABLE Times (
    "id"                SERIAL PRIMARY KEY,
    "class_id"          INT REFERENCES Classes("class_id")
    "day"               VARCHAR(50) NOT NULL,
    "time"              VARCHAR(50) NOT NULL,
    "location"          VARCHAR(255) NOT NULL,
    "weeks"             VARCHAR(255) NOT NULL,
    "instructor"        VARCHAR(255)
);

CREATE TABLE Enrolments (
    "id"                SERIAL PRIMARY KEY,
    "class_id"          INT REFERENCES Classes("class_id"),
    "enrolled"          INT NOT NULL,
    "capacity"          INT NOT NULL
);




