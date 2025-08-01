// Re-export modules and types for use in tests
pub mod db;
pub mod openapi;
pub mod holidays_api;

use actix_web::{web, HttpResponse, post};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, TimeZone, Datelike, NaiveDate};
use chrono_tz::Tz;
use std::sync::Mutex;
use actix_web::cookie::time::Time;

// Global variables to store the start and end of day times

// Re-export types and functions needed for tests
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Holiday {
    pub date: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WorkHoursRequest {
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(flatten)]
    #[serde(default)]
    pub end_or_duration: EndOrDuration,
    #[serde(rename = "startOfDay", default = "default_start_of_day")]
    pub start_of_day: String,
    #[serde(rename = "endOfDay", default = "default_end_of_day")]
    pub end_of_day: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub subdivision: Option<String>,
    #[serde(default)]
    pub timezone: String,
}

fn default_start_of_day() -> String {
    "09:00:00".to_string()
}

fn default_end_of_day() -> String {
    "17:00:00".to_string()
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

impl Default for EndOrDuration {
    fn default() -> Self {
        EndOrDuration::EndDate {
            end_date: "".to_string()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkHoursResponse {
    pub work_hours: f64,
    pub work_minutes: f64,
    pub work_seconds: f64,
    pub start_date: String,
    pub end_date: String,
}

pub struct AppState {
    pub db: Mutex<db::Database>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkHoursQueryParams {
    #[serde(rename = "startDate")]
    pub start_date: String,
    #[serde(rename = "endDate")]
    pub end_date: Option<String>,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: Option<i64>,
    #[serde(rename = "startOfDay", default = "default_start_of_day")]
    pub start_of_day: String,
    #[serde(rename = "endOfDay", default = "default_end_of_day")]
    pub end_of_day: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub subdivision: Option<String>,
}

#[post("/")]
pub async fn get_work_hours(
    data: web::Data<AppState>,
    workhours: web::Json<WorkHoursQueryParams>,
) -> Result<HttpResponse, actix_web::error::Error> {
    log::debug!("Received work hours request: {:?}", workhours);

    // Convert query params to WorkHoursRequest
    let request = WorkHoursRequest {
        start_date: workhours.start_date.clone(),
        end_or_duration: if let Some(end_date) = &workhours.end_date {
            EndOrDuration::EndDate { 
                end_date: end_date.clone() 
            }
        } else if let Some(duration_seconds) = workhours.duration_seconds {
            EndOrDuration::Duration { 
                duration_seconds 
            }
        } else {
            return Ok(HttpResponse::BadRequest().json("Either endDate or durationSeconds must be provided"));
        },
        start_of_day: workhours.start_of_day.clone(),
        end_of_day: workhours.end_of_day.clone(),
        country: workhours.country.clone(),
        timezone: workhours.timezone.clone(),
        subdivision: workhours.subdivision.clone()
    };

    calculate_work_hours(data, web::Json(request)).await
}

pub async fn calculate_work_hours(
    data: web::Data<AppState>,
    req: web::Json<WorkHoursRequest>,
) -> Result<HttpResponse, actix_web::error::Error> {
    log::debug!("Processing work hours calculation: {:?}", req);
    // Parse dates and convert to timezone-aware datetimes
    let start_date = DateTime::parse_from_rfc3339(&req.start_date)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid start date format: {}", e)))?;
    let time_format = actix_web::cookie::time::format_description::parse("[hour]:[minute]:[second]").unwrap();
    let start_of_day = Time::parse(&req.start_of_day, &time_format)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid start time format: {}", e)))?;
    let end_of_day = Time::parse(&req.end_of_day, &time_format)
        .map_err(|e| actix_web::error::ErrorBadRequest(format!("Invalid end time format: {}", e)))?;
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
    let subdivision = req.subdivision.clone().unwrap_or(String::new());
    let mut work_hours = 0.0;


    let mut current = start_date;

    // Fetch holidays from API instead of database
    let holidays = if cfg!(test) {
        // In test mode, use the mock implementation
        match holidays_api::mock::get_holidays_for_country(&country, &subdivision).await {
            Ok(api_holidays) => {
                // Convert API holidays to the format expected by the work hours calculation
                holidays_api::convert_to_db_holiday(api_holidays, &country)
            },
            Err(e) => {
                // Log the error and fall back to database
                log::error!("Failed to fetch holidays from mock API: {}. Falling back to database.", e);
                let db = data.db.lock().unwrap();
                db.get_holidays_by_country(&country).unwrap_or(vec![])
            }
        }
    } else {
        // In production mode, use the real implementation
        match holidays_api::get_holidays_for_country(&country, &subdivision, current.date_naive()).await {
            Ok(api_holidays) => {
                // Convert API holidays to the format expected by the work hours calculation
                holidays_api::convert_to_db_holiday(api_holidays, &country)
            },
            Err(e) => {
                // Log the error and fall back to database
                log::error!("Failed to fetch holidays from API: {}. Falling back to database.", e);
                let db = data.db.lock().unwrap();
                db.get_holidays_by_country(&country).unwrap_or(vec![])
            }
        }
    };

    while current.date_naive() <= end_date.date_naive() {
        // Skip weekends and holidays
        if current.weekday() == chrono::Weekday::Sat || current.weekday() == chrono::Weekday::Sun {
            current = current + chrono::Duration::days(1);
            continue;
        }
        // Check if current date is a holiday
        let is_holiday = {
            holidays.iter().any(|h| {
                println!("raw date from open data: {:?}", h.date);
                let h_date = NaiveDate::parse_from_str(&h.date[..10], "%Y-%m-%d").unwrap();
                h_date == current.date_naive()
            })
        };

        if is_holiday {
            current = current + chrono::Duration::days(1);
            continue;
        }

        // Get start and end of day times from global variables

        let full_day = (end_of_day - start_of_day).as_seconds_f64() / 3600.0;

        // Create start and end of day datetimes
        let start_of_day = timezone.from_local_datetime(&current.date_naive().and_hms_opt(start_of_day.hour() as u32, start_of_day.minute() as u32, start_of_day.second() as u32).unwrap()).unwrap();
        let end_of_day = timezone.from_local_datetime(&current.date_naive().and_hms_opt(end_of_day.hour() as u32, end_of_day.minute() as u32, end_of_day.second() as u32).unwrap()).unwrap();
        if current.date_naive() == start_date.date_naive() && current.date_naive() == end_date.date_naive() {
            if start_date.time() > end_of_day.time() || end_date.time() < start_of_day.time()   {
                current = current + chrono::Duration::days(1);
                continue;
            }
            let effective_start = if start_date.time() < start_of_day.time() {
                start_of_day
            } else {
                start_date
            };
            let effective_end = if end_date.time() > end_of_day.time() {
                end_of_day
            } else {
                end_date
            };
            work_hours += effective_end.signed_duration_since(effective_start).num_seconds() as f64 / 3600.0;
        }
        else if current.date_naive() == start_date.date_naive() {
            // Start date - special case for standard start time
            if start_date.time() >= end_of_day.time()   {
                current = current + chrono::Duration::days(1);
                continue;
            }
            // Partial start day
            let effective_start = if start_date.time() < start_of_day.time() {
                start_of_day
            } else {
                start_date
            };

            work_hours += end_of_day.signed_duration_since(effective_start).num_seconds() as f64 / 3600.0
        }
        else if current.date_naive() == end_date.date_naive() {
            // End date - special case for standard end time
            if end_date.time() < start_of_day.time()    {
                current = current + chrono::Duration::days(1);
                continue;
            }
            let effective_end = if end_date.time() > end_of_day.time() {
                end_of_day
            } else {
                end_date
            };

            work_hours += effective_end.signed_duration_since(start_of_day).num_seconds() as f64 / 3600.0
        }

        else {
            // Full workday - always 8 hours
            work_hours += full_day;
        }

        current = current + chrono::Duration::days(1);
    }

    Ok(HttpResponse::Ok().json(WorkHoursResponse {
        work_hours,
        work_minutes: work_hours * 60.0,
        work_seconds: work_hours * 3600.0,
        start_date: start_date.to_rfc3339(),
        end_date: end_date.to_rfc3339(),
    }))
}

// Unit tests for the library
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::web;
    use holidays_api::mock as holidays_api_mock;

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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
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

        // Set up mock holidays for the API
        holidays_api_mock::set_mock_holidays("us", vec![
            holidays_api::Holiday {
                date: "2023-10-04T00:00:00Z".to_string(),
                description: "Test Holiday".to_string(),
            },
        ]);

        // Also set up database holidays as fallback
        let db_data = create_test_db_with_holidays(vec![
            ("2023-10-04T00:00:00Z".to_string(), "Test Holiday".to_string(), "us".to_string()),
        ]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T09:00:00Z".to_string(), // Monday
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-06T17:00:00Z".to_string() // Friday
            },
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 32.0);

        // Clean up mock holidays
        holidays_api_mock::clear_mock_holidays();
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "fr".to_string(),
            timezone: "Europe/Paris".to_string(),
            subdivision: None,
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
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
            start_of_day: default_start_of_day(),
            end_of_day: default_end_of_day(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await;
        assert!(result.is_err(), "Expected an error when duration is negative");

        if let Err(e) = result {
            let error_string = format!("{}", e);
            assert!(error_string.contains("Start date must be strictly before end date"), 
                   "Error message should mention that start date must be before end date");
        }
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_with_custom_day_times() {
        // Test work hours calculation with custom start and end of day times
        // 8am to 4pm instead of 9am to 5pm
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T08:00:00Z".to_string(), // Monday at 8am
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-02T16:00:00Z".to_string() // Monday at 4pm
            },
            start_of_day: "08:00:00".to_string(),
            end_of_day: "16:00:00".to_string(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 8.0);
    }

    #[actix_rt::test]
    async fn test_calculate_work_hours_with_partial_custom_day() {
        // Test work hours calculation with custom start and end of day times
        // Starting at 10am with custom day from 8am to 4pm
        let db_data = create_test_db_with_holidays(vec![]);

        let request = WorkHoursRequest {
            start_date: "2023-10-02T10:00:00Z".to_string(), // Monday at 10am
            end_or_duration: EndOrDuration::EndDate { 
                end_date: "2023-10-02T16:00:00Z".to_string() // Monday at 4pm
            },
            start_of_day: "08:00:00".to_string(),
            end_of_day: "16:00:00".to_string(),
            country: "us".to_string(),
            timezone: "UTC".to_string(),
            subdivision: None,
        };

        let result = calculate_work_hours(db_data, web::Json(request)).await.unwrap();
        let body = result.into_body();
        let bytes = actix_web::body::to_bytes(body).await.unwrap();
        let response: WorkHoursResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(response.work_hours, 6.0); // 8am to 4pm = 8 hours

    }
}
