FROM rust:1.74.1

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=9953

WORKDIR /app
COPY . .
RUN cargo install --path .

EXPOSE 9953
CMD ["cargo", "run"]