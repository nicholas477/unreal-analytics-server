FROM rust:1.74.1

ENV ROCKET_CONFIG=/app/config/Rocket.toml
WORKDIR /app
COPY . .
RUN cargo install --path .

EXPOSE 9953
CMD ["cargo", "run"]