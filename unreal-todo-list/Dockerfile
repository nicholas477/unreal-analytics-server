FROM rust:1.74.1

WORKDIR /app
COPY . .
RUN cargo install --path .

EXPOSE 9092
CMD ["cargo", "run"]