CREATE TABLE Times (
    "time_id"                         VARCHAR(511) PRIMARY KEY,
    "class_id"                        VARCHAR(255) NOT NULL,
    "day"                             VARCHAR(255) NOT NULL,
    "instructor"                      VARCHAR(255),
    "location"                        VARCHAR(255) NOT NULL,
    "time"                            VARCHAR(255) NOT NULL,
    "weeks"                           VARCHAR(255) NOT NULL,
    "career"                          VARCHAR(255) NOT NULL,
    FOREIGN KEY ("class_id") REFERENCES Classes("class_id") ON DELETE CASCADE
);
