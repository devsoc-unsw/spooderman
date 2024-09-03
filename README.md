# spooderman

The scraper is written to be versatile to changes in the timetable website.
UNSW is notorious to change and a lot of new nuances could be added without the team
knowing. This scraper is designed to be super fast and accurate in parsing recent timetable data.

## Instructions to run:

There are couple of things you must ensure.
The scraper was written to be batch inserted into Hasuragres. (Look at hasuragres / GraphQL API).
The course data is scraped from the UNSW timetable website https://timetable.unsw.edu.au/year/.

If you run `cargo run -- help`, it will give a list of commands you can run.

Usage:
scrape - Perform scraping. Creates a json file to store the data.
scrape_n_batch_insert - Perform scraping and batch insert. Does not create a json file to store the data.
batch_insert - Perform batch insert on json files created by scrape.
help - Show this help message
