// Re-export modules and types for use in tests
pub mod db;
pub mod openapi;

use actix_web::{web, HttpResponse, get, post, Responder};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, TimeZone, Datelike};
use chrono_tz::Tz;
use std::sync::Mutex;

// Re-export types and functions needed for tests
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Holiday {
    pub date: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkHoursRequest {
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(flatten)]
    pub end_or_duration: EndOrDuration,
    pub country: String,
    pub timezone: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EndOrDuration {
    EndDate { 
        #[serde(alias = "endDate")]
        end_date: String 
    },
    Duration { 
        #[serde(alias = "durationSeconds")]
        duration_seconds: i64 
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkHoursResponse {
    pub work_hours: f64,
    pub start_date: String,
    pub end_date: String,
}

pub struct AppState {
    pub db: Mutex<db::Database>,
}

// Re-export the handler functions for testing
#[post("/holidays/{country}")]
pub async fn add_holiday(
    data: web::Data<AppState>,
    holiday: web::Json<Holiday>,
    country: web::Path<String>,
) -> impl Responder {
    let country = country.to_lowercase();
    let holiday = db::Holiday {
        id: None,
        date: holiday.date.clone(),
        description: holiday.description.clone(),
        country: country.clone(),
    };

    let db = data.db.lock().unwrap();
    match db.add_holiday(&holiday) {
        Ok(_) => HttpResponse::Ok().json("Holiday added successfully"),
        Err(e) => HttpResponse::InternalServerError().json(format!("Failed to add holiday: {}", e)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkHoursQueryParams {
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    pub country: String,
    pub timezone: String,
}

#[get("/")]
pub async fn get_work_hours(
    data: web::Data<AppState>,
    req: web::Query<WorkHoursQueryParams>,
) -> Result<HttpResponse, actix_web::error::Error> {
    log::debug!("Received work hours request: {:?}", req);

    // Convert query params to WorkHoursRequest
    let request = WorkHoursRequest {
        start_date: req.start_date.clone(),
        end_or_duration: if let Some(end_date) = &req.end_date {
            EndOrDuration::EndDate { 
                end_date: end_date.clone() 
            }
        } else if let Some(duration_seconds) = req.duration_seconds {
            EndOrDuration::Duration { 
                duration_seconds 
            }
        } else {
            return Ok(HttpResponse::BadRequest().json("Either endDate or durationSeconds must be provided"));
        },
        country: req.country.clone(),
        timezone: req.timezone.clone(),
    };

    calculate_work_hours(data, web::Json(request)).await
}

#[get("/holidays/{country}")]
pub async fn list_holidays(
    data: web::Data<AppState>,
    country: web::Path<String>,
) -> impl Responder {
    let country = country.to_lowercase();
    let db = data.db.lock().unwrap();
    match db.get_holidays_by_country(&country) {
        Ok(holidays) => Ok::<HttpResponse, actix_web::error::Error>(HttpResponse::Ok().json(holidays)),
        Err(e) => Err(actix_web::error::ErrorInternalServerError(format!("Failed to fetch holidays: {}", e))),
    }
}

pub async fn calculate_work_hours(
    data: web::Data<AppState>,
    req: web::Json<WorkHoursRequest>,
) -> Result<HttpResponse, actix_web::error::Error> {
    log::debug!("Processing work hours calculation: {:?}", req);
    // Parse dates and convert to timezone-aware datetimes
    let start_date = DateTime::parse_from_rfc3339(&req.start_date)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid start date format: {}", e)))?;
    let timezone: Tz = req.timezone.parse()
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid timezone: {}", e)))?;

    // Convert to timezone-aware datetime
    let start_date = timezone.from_local_datetime(&start_date.naive_local()).unwrap();

    let (end_date, _duration_seconds) = match &req.end_or_duration {
        EndOrDuration::EndDate { end_date } => {
            let end_date = DateTime::parse_from_rfc3339(end_date)
                .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid end date format: {}", e)))?;
            let end_date = timezone.from_local_datetime(&end_date.naive_local()).unwrap();
            (end_date, (end_date - start_date).num_seconds())
        }
        EndOrDuration::Duration { duration_seconds } => {
            let end_date = start_date + chrono::Duration::seconds(*duration_seconds);
            (end_date, *duration_seconds)
        }
    };

    // Validate that start date is strictly before end date
    if start_date >= end_date {
        return Err(actix_web::error::ErrorBadRequest("Start date must be strictly before end date"));
    }

    let country = req.country.to_lowercase();
    let mut work_hours = 0.0;


    let mut current = start_date;
    while current.date_naive() <= end_date.date_naive() {
        // Skip weekends and holidays
        if current.weekday() == chrono::Weekday::Sat || current.weekday() == chrono::Weekday::Sun {
            current = current + chrono::Duration::days(1);
            continue;
        }

        // Check if current date is a holiday
        let is_holiday = {
            let db = data.db.lock().unwrap();
            let holidays = db.get_holidays_by_country(&country).unwrap_or(vec![]);
            holidays.iter().any(|h| {
                let h_date = DateTime::parse_from_rfc3339(&h.date)
                    .map(|d| timezone.from_local_datetime(&d.naive_local()).unwrap())
                    .unwrap_or(current);
                h_date.date_naive() == current.date_naive()
            })
        };

        if is_holiday {
            current = current + chrono::Duration::days(1);
            continue;
        }

        // Count work hours (9-17)
        let start_of_day = timezone.from_local_datetime(&current.date_naive().and_hms_opt(9, 0, 0).unwrap()).unwrap();
        let end_of_day = timezone.from_local_datetime(&current.date_naive().and_hms_opt(17, 0, 0).unwrap()).unwrap();

        if current.date_naive() == end_date.date_naive() {
            let day_work_hours = if end_date.time() < start_of_day.time() {
                0.0
            } else if end_date.time() >= start_of_day.time() && end_date.time() <= end_of_day.time() {
                (end_date.signed_duration_since(start_of_day)).num_seconds() as f64 / 3600.0
            } else {
                8.0
            };
            work_hours += day_work_hours;
        }
        else if current.date_naive() == start_date.date_naive() {
            let day_work_hours = if start_date.time() < start_of_day.time() {
                8.0
            } else if start_date.time() >= start_of_day.time() && start_date.time() <= end_of_day.time() {
                end_of_day.signed_duration_since(start_date).num_seconds() as f64 / 3600.0
            } else {
                0.0
            };
            work_hours += day_work_hours;
        }
        else {
            work_hours += 8.0;
        }

        current = current + chrono::Duration::days(1);
    }

    Ok(HttpResponse::Ok().json(WorkHoursResponse {
        work_hours,
        start_date: start_date.to_rfc3339(),
        end_date: end_date.to_rfc3339(),
    }))
}

// Unit tests for the library
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web;

    #[test]
    fn test_end_or_duration_deserialization() {
        // Test EndDate variant
        let json = r#"{"end_date": "2023-10-06T17:00:00Z"}"#;
        let end_or_duration: EndOrDuration = serde_json::from_str(json).unwrap();
        match end_or_duration {
            EndOrDuration::EndDate { end_date } => assert_eq!(end_date, "2023-10-06T17:00:00Z"),
            _ => panic!("Expected EndDate variant"),
        }

        // Test Duration variant
        let json = r#"{"duration_seconds": 86400}"#;
        let end_or_duration: EndOrDuration = serde_json::from_str(json).unwrap();
        match end_or_duration {
            EndOrDuration::Duration { duration_seconds } => assert_eq!(duration_seconds, 86400),
            _ => panic!("Expected Duration variant"),
        }
    }

    #[test]
    fn test_holiday_serialization() {
        let holiday = Holiday {
            date: "2023-12-25T00:00:00Z".to_string(),
            description: "Christmas".to_string(),
        };

        let json = serde_json::to_string(&holiday).unwrap();
        let expected = r#"{"date":"2023-12-25T00:00:00Z","description":"Christmas"}"#;
        assert_eq!(json, expected);

        let deserialized: Holiday = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.date, holiday.date);
        assert_eq!(deserialized.description, holiday.description);
    }

    // Helper function to create a test database with holidays
    fn create_test_db_with_holidays(holidays: Vec<(String, String, String)>) -> web::Data<AppState> {
        let db = db::Database::new(":memory:").unwrap();

        for (date, description, country) in holidays {
            let holiday = db::Holiday {
                id: None,
                date,
                description,
                country,
            };
            db.add_holiday(&holiday).unwrap();
        }

        web::Data::new(AppState {
            db: Mutex::new(db),
        })
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_weekday() {
        // Test a single workday (Monday)
        // 8 hours of work (9am to 5pm)
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00Z".to_string(),
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-02T17:00:00Z".to_string() 
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 8.0);
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_weekend() {
        // Test a weekend (Saturday and Sunday)
        // 0 hours of work
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-07T09:00:00Z".to_string(), // Saturday
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-08T17:00:00Z".to_string() // Sunday
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 0.0);
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_with_holiday() {
        // Test a workweek with a holiday
        // Monday to Friday, but Wednesday is a holiday
        // 4 days * 8 hours = 32 hours
        let db_data = create_test_db_with_holidays(vec![
            ("2023-10-04T00:00:00Z".to_string(), "Test Holiday".to_string(), "us".to_string()),
        ]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00Z".to_string(), // Monday
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-06T17:00:00Z".to_string() // Friday
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 32.0);
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_partial_day() {
        // Test a partial workday
        // Starting at 12pm instead of 9am (3 hours less)
        // 5 hours of work
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T12:00:00Z".to_string(),
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-02T17:00:00Z".to_string()
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 5.0);
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_different_timezone() {
        // Test work hours calculation with a different timezone
        // 9am to 5pm in Europe/Paris
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00+02:00".to_string(), // 9am Paris time
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-02T17:00:00+02:00".to_string() // 5pm Paris time
            },
            country: "fr".to_string(),
            timezone: "Europe/Paris".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 8.0);
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_with_duration() {
        // Test work hours calculation with duration
        // Starting Monday 9am, duration 5 days
        // Monday to Friday = 5 workdays * 8 hours = 40 hours
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00Z".to_string(), // Monday
            end_or_duration: EndOrDuration::Duration { 
                duration_seconds: 432000 // 5 days
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 40.0);
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_invalid_date_range_equal() {
        // Test case where start_date equals end_date
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00Z".to_string(),
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-02T09:00:00Z".to_string() // Same as start_date
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await;
        assert!(result.is_err(), "Expected an error when start_date equals end_date");

        if let Err(e) = result {
            let error_string = format!("{}", e);
            assert!(error_string.contains("Start date must be strictly before end date"), 
                   "Error message should mention that start date must be before end date");
        }
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_invalid_date_range_after() {
        // Test case where start_date is after end_date
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-03T09:00:00Z".to_string(), // Tuesday
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-02T09:00:00Z".to_string() // Monday (before start_date)
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await;
        assert!(result.is_err(), "Expected an error when start_date is after end_date");

        if let Err(e) = result {
            let error_string = format!("{}", e);
            assert!(error_string.contains("Start date must be strictly before end date"), 
                   "Error message should mention that start date must be before end date");
        }
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_with_zero_duration() {
        // Test case where duration is zero, resulting in start_date equals end_date
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00Z".to_string(),
            end_or_duration: EndOrDuration::Duration { 
                duration_seconds: 0 // Zero duration
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await;
        assert!(result.is_err(), "Expected an error when duration is zero");

        if let Err(e) = result {
            let error_string = format!("{}", e);
            assert!(error_string.contains("Start date must be strictly before end date"), 
                   "Error message should mention that start date must be before end date");
        }
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_with_negative_duration() {
        // Test case where duration is negative, resulting in end_date before start_date
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00Z".to_string(),
            end_or_duration: EndOrDuration::Duration { 
                duration_seconds: -86400 // Negative duration (1 day)
            },
            country: "us".to_string(),
            timezone: "UTC".to_string(),
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await;
        assert!(result.is_err(), "Expected an error when duration is negative");

        if let Err(e) = result {
            let error_string = format!("{}", e);
            assert!(error_string.contains("Start date must be strictly before end date"), 
                   "Error message should mention that start date must be before end date");
        }
    }
}
