FROM rust:1.80 AS builder

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/spooderman /usr/local/bin/spooderman
COPY --from=builder /app/sql ./sql

ENV TIMETABLE_API_URL=https://timetable.unsw.edu.au/year/

ENTRYPOINT ["spooderman"]
CMD ["scrape_n_batch_insert", "--year", "latest-with-data"]
