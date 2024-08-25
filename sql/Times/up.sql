CREATE TABLE Times (
    "id"                              VARCHAR(500) PRIMARY KEY,
    "class_id"                        VARCHAR(255) NOT NULL,
    "course_id"                       VARCHAR(255) NOT NULL,
    "day"                             VARCHAR(255) NOT NULL,
    "instructor"                      VARCHAR(255),
    "location"                        VARCHAR(255) NOT NULL,
    "time"                            VARCHAR(100) NOT NULL,
    "weeks"                           VARCHAR(100) NOT NULL,
    FOREIGN KEY ("class_id") REFERENCES Classes("class_id") ON DELETE CASCADE,
    FOREIGN KEY ("course_id") REFERENCES Courses("subject_area_course_code") ON DELETE CASCADE
);
