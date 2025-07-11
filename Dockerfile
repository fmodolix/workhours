FROM rust:1.88-bullseye as builder

WORKDIR /usr/src/workhours
COPY . .

RUN cargo build --release

FROM debian:bullseye-slim

RUN apt update && apt install -y \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/workhours/target/release/workhours /app/
COPY swagger-ui.html /app/

# Create a directory for the database
RUN mkdir -p /data

# Set environment variables
ENV DATABASE_LOCATION=/data/workhours.db
ENV SERVER_HOST=127.0.0.1
ENV SERVER_PORT=8080

EXPOSE 8080

CMD ["./workhours"]