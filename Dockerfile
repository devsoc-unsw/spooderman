FROM rust:1.80
WORKDIR /app
COPY . .

ARG HASURAGRES_URL
ARG HASURAGRES_API_KEY

ENV TIMETABLE_API_URL=https://timetable.unsw.edu.au/year/

RUN cargo r -- scrape_n_batch_insert -release

