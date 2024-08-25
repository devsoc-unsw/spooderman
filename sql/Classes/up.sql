CREATE TYPE status_enum AS ENUM ('Open', 'Closed');
CREATE TABLE Classes (
    "class_id"                        VARCHAR(255) PRIMARY KEY,
    "course_id"                       VARCHAR(8)   NOT NULL,
    "section"                         VARCHAR(255) NOT NULL,
    "term"                            VARCHAR(50)  NOT NULL, 
    "activity"                        VARCHAR(255) NOT NULL,
    "status"                          status_enum,
    "course_enrolment"                VARCHAR(255) NOT NULL,
    "offering_period"                 VARCHAR(255) NOT NULL,
    "meeting_dates"                   VARCHAR(255) NOT NULL,
    "census_date"                     VARCHAR(255) NOT NULL,
    "consent"                         VARCHAR(255) NOT NULL,
    "mode"                            VARCHAR(255) NOT NULL,
    "class_notes"                     TEXT,
    -- "times"                           TEXT,
    FOREIGN KEY ("course_id") REFERENCES Courses("subject_area_course_code") ON DELETE CASCADE
);