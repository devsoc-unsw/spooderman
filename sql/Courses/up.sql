CREATE TABLE Courses (
    "course_code"     VARCHAR(8) PRIMARY KEY, --id
    "year"            VARCHAR(4)   NOT NULL,                           
    "course_name"     VARCHAR(255) NOT NULL,
    "uoc"             INT          NOT NULL,
    "faculty"         VARCHAR(255),
    "school"          VARCHAR(255),
    "campus"          VARCHAR(255),
    "career"          VARCHAR(255),
    "terms"           TEXT,
    "modes"           VARCHAR(255)[]
);