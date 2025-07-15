use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use chrono::{NaiveDate, Datelike};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use log::{error, info};

// For testing
use std::cell::RefCell;

// Define the Holiday struct to match our existing Holiday struct
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Holiday {
    pub date: String,
    #[serde(default)]
    pub description: String,
}

// Struct to represent the API response
#[derive(Debug, Deserialize)]
struct OpenHolidayApiResponse {
    #[serde(rename = "startDate")]
    start_date: String,
    name: Vec<LocalizedName>
}

#[derive(Debug, Deserialize)]
struct LocalizedName {
    language: String,
    text: String,
}

// Cache entry with expiration time
struct CacheEntry {
    holidays: Vec<Holiday>,
    expiration: SystemTime,
}

// Global cache for holidays
lazy_static! {
    static ref HOLIDAY_CACHE: Mutex<HashMap<String, CacheEntry>> = Mutex::new(HashMap::new());
}

// Function to get holidays for a country from the API
pub async fn get_holidays_for_country(country: &str, subdivision: &str, current_date: NaiveDate) -> Result<Vec<Holiday>, String> {
    // Check if we have a valid cached entry
    let code = if subdivision != "" {
        &subdivision.to_uppercase()
    } else {
        &country.to_uppercase()
    };
    let cache_key = code.to_string() + current_date.year().to_string().as_str();
    {
        let cache = HOLIDAY_CACHE.lock().unwrap();

        if let Some(entry) = cache.get(&cache_key) {
            let now = SystemTime::now();
            if entry.expiration > now {
                let remaining_secs = entry.expiration.duration_since(now).unwrap_or(Duration::from_secs(0)).as_secs();
                info!("CACHE HIT: Using cached holidays for key: {}. Cache expires in {} seconds", cache_key, remaining_secs);
                return Ok(entry.holidays.clone());
            } else {
                info!("CACHE EXPIRED: Holidays cache for country: {} has expired", code);
            }
        } else {
            info!("CACHE MISS: No cached holidays found for country: {}", code);
        }
    }

    // If not in cache or expired, fetch from API
    info!("Fetching holidays from API for country: {}", country);

    // Get current year
    let current_year = current_date.year();

    // Construct the API URL

    let url = if subdivision != "" {
        format!(
            "https://openholidaysapi.org/PublicHolidays?countryIsoCode={}&subdivisionCode={}&languageIsoCode=EN&validFrom={}-01-01&validTo={}-12-31",
            country.to_uppercase(),
            subdivision.to_uppercase(),
            current_year,
            current_year + 1
        )
    } else {
        format!(
            "https://openholidaysapi.org/PublicHolidays?countryIsoCode={}&languageIsoCode=EN&validFrom={}-01-01&validTo={}-12-31",
            country.to_uppercase(),
            current_year,
            current_year + 1
        )
    };

    // Make the API request
    let response = match reqwest::get(&url).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to fetch holidays from API: {}", e);
            return Err(format!("Failed to fetch holidays: {}", e));
        }
    };

    // Check if the request was successful
    if !response.status().is_success() {
        let status = response.status();
        error!("API request failed with status: {}", status);
        return Err(format!("API request failed with status: {}", status));
    }

    // Parse the response
    let api_holidays: Vec<OpenHolidayApiResponse> = match response.json().await {
        Ok(holidays) => holidays,
        Err(e) => {
            error!("Failed to parse API response: {}", e);
            return Err(format!("Failed to parse API response: {}", e));
        }
    };

    // Convert API holidays to our Holiday format
    let holidays: Vec<Holiday> = api_holidays
        .into_iter()
        .map(|api_holiday| {
            let description = api_holiday.name
                .iter()
                .find(|name| name.language == "EN")
                .map_or_else(
                    || api_holiday.name.first().map_or("".to_string(), |name| name.text.clone()),
                    |name| name.text.clone()
                );

            Holiday {
                date: api_holiday.start_date,
                description,
            }
        })
        .collect();

    // Cache the result with 24-hour expiration
    {
        let cache_duration = Duration::from_secs(24 * 60 * 60);
        let expiration_time = SystemTime::now() + cache_duration;

        let mut cache = HOLIDAY_CACHE.lock().unwrap();

        cache.insert(
            cache_key.clone(),
            CacheEntry {
                holidays: holidays.clone(),
                expiration: expiration_time,
            },
        );
        info!("CACHE UPDATE: Cached {} holidays for key: {}. Cache will expire in {} seconds",
              holidays.len(), cache_key, cache_duration.as_secs());
    }

    Ok(holidays)
}

// Function to convert our Holiday format to the format expected by the work hours calculation
pub fn convert_to_db_holiday(holidays: Vec<Holiday>, country: &str) -> Vec<crate::db::Holiday> {
    holidays
        .into_iter()
        .map(|holiday| crate::db::Holiday {
            id: None,
            date: holiday.date,
            description: holiday.description,
            country: country.to_string(),
        })
        .collect()
}

// Mock implementation for testing
pub mod mock {
    use super::*;

    thread_local! {
        static MOCK_HOLIDAYS: RefCell<HashMap<String, Vec<Holiday>>> = RefCell::new(HashMap::new());
    }

    pub fn set_mock_holidays(country: &str, holidays: Vec<Holiday>) {
        MOCK_HOLIDAYS.with(|mock_holidays| {
            mock_holidays.borrow_mut().insert(country.to_string(), holidays);
        });
    }

    pub fn clear_mock_holidays() {
        MOCK_HOLIDAYS.with(|mock_holidays| {
            mock_holidays.borrow_mut().clear();
        });
    }

    pub async fn get_holidays_for_country(country: &str, subdivision: &str) -> Result<Vec<Holiday>, String> {
        let code = if subdivision != "" {
            &subdivision.to_uppercase()
        } else {
            &country.to_uppercase()
        };
        MOCK_HOLIDAYS.with(|mock_holidays| {
            let holidays = mock_holidays.borrow();
            match holidays.get(code) {
                Some(holidays) => {
                    info!("MOCK: Using mock holidays for country: {}, found {} holidays", country, holidays.len());
                    Ok(holidays.clone())
                },
                None => {
                    error!("MOCK: No mock holidays found for country: {}", country);
                    Err(format!("No mock holidays found for country: {}", country))
                },
            }
        })
    }
}
