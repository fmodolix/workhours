# Work Hours API

A Rust-based API for calculating work hours between dates, taking into account weekends, holidays, and timezones.

## Features

- Calculate work hours between two dates
- Support for different timezones
- Handling of weekends and holidays
- RESTful API with Swagger documentation

## Docker Setup

This project includes Docker configuration for easy deployment.

### Prerequisites

- Docker
- Docker Compose

### Building and Running with Docker

1. Clone the repository:
   ```
   git clone <repository-url>
   cd workhours
   ```

2. Build and start the container:
   ```
   docker-compose up -d
   ```

   This will:
   - Build the Docker image
   - Start the container
   - Mount a volume for the database at `/data` inside the container
   - Expose the API on port 8080

3. Access the API:
   - API: http://localhost:8080/
   - Swagger UI: http://localhost:8080/swagger

### Database Persistence

The SQLite database is stored on a mounted volume (`workhours_data`) to ensure data persistence between container restarts. The database file is located at `/data/workhours.db` inside the container.

### Environment Variables

The following environment variables can be configured in the `docker-compose.yml` file:

- `DATABASE_LOCATION`: Path to the SQLite database file (default: `/data/workhours.db`)
- `SERVER_HOST`: Host address for the server to listen on (default: `0.0.0.0`)
- `SERVER_PORT`: Port for the server to listen on (default: `8080`)

## API Usage

### Calculate Work Hours

```
GET /?startDate=2023-10-02T09:00:00Z&endDate=2023-10-06T17:00:00Z&country=us&timezone=UTC
```

### Add Holidays

```
POST /holidays/us
Content-Type: application/json

[
  {
    "date": "2023-12-25T00:00:00Z",
    "description": "Christmas"
  }
]
```

### List Holidays

```
GET /holidays/us
```

## Development

To run the project locally without Docker:

1. Install Rust and Cargo
2. Clone the repository
3. Set up environment variables in `.env` file
4. Run the project:
   ```
   cargo run
   ```

5. Run tests:
   ```
   cargo test
   ```