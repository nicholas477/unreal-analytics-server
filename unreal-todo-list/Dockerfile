FROM rust:latest

ENV ROCKET_CONFIG=/app/config/Rocket.toml
ENV RUST_BACKTRACE=1
WORKDIR /app
COPY . .
RUN cargo install --path .

EXPOSE 9092
CMD ["cargo", "run"]