FROM rust:1.76.0 AS builder

RUN mkdir /code
WORKDIR /code
# Pre-build a fake rust project. Should cache unless we change the Cargo.toml
COPY ./Cargo.toml .
RUN mkdir src
RUN echo "fn main() {}" > src/main.rs
RUN cargo build --release

# Install cron
RUN apt-get update && apt-get -y install cron
RUN touch /var/log/cron.log
# Copy cron file
COPY ./cronfile /etc/cron.d/feed_cron
RUN chmod +x /etc/cron.d/feed_cron

# Copy again and build the code
COPY . .
RUN cargo install --path .

# Command to run
CMD cron && tail -f /var/log/cron.log
