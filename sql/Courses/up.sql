CREATE TABLE Courses (
    "subject_area_course_code"        VARCHAR(8) PRIMARY KEY, --id
    "subject_area_course_name"        VARCHAR(50) NOT NULL,
    "uoc"                             INT NOT NULL,
    "faculty"                         VARCHAR(50),
    "school"                          VARCHAR(50),
    "campus"                          VARCHAR(50),
    "career"                          VARCHAR(50),
    "terms"                           VARCHAR(50)[]
);