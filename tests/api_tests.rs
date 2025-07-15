use actix_web::{test, web, App};
use actix_http;
use workhours::{
    add_holiday, get_work_hours, list_holidays, 
    Holiday, WorkHoursResponse
};
use log::info;

// Helper function to create a test app
async fn create_test_app() -> impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
> {
    // Create an in-memory database
    let db = workhours::db::Database::new(":memory:").unwrap();

    // Create the AppState with the database wrapped in a Mutex
    let app_state = web::Data::new(workhours::AppState {
        db: std::sync::Mutex::new(db),
    });

    test::init_service(
        App::new()
            .app_data(app_state.clone())
            .service(add_holiday)
            .service(get_work_hours)
            .service(list_holidays)
    ).await
}

#[actix_rt::test]
async fn test_add_holiday() {
    let app = create_test_app().await;

    // Create a test holiday list
    let holidays = vec![
        Holiday {
            date: "2023-12-25T00:00:00Z".to_string(),
            description: "Christmas".to_string(),
        }
    ];

    // Send POST request to add the holiday
    let resp = test::TestRequest::post()
        .uri("/holidays/us")
        .set_json(&holidays)
        .send_request(&app)
        .await;

    // Check response
    assert_eq!(resp.status(), 200);

    // Verify the holiday was added by listing holidays
    let resp = test::TestRequest::get()
        .uri("/holidays/us")
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 200);

    let holidays: Vec<workhours::db::Holiday> = test::read_body_json(resp).await;
    assert_eq!(holidays.len(), 1);
    assert_eq!(holidays[0].date, "2023-12-25T00:00:00Z");
    assert_eq!(holidays[0].description, "Christmas");
    assert_eq!(holidays[0].country, "us");
}

#[actix_rt::test]
async fn test_list_holidays() {
    let app = create_test_app().await;

    // Add a test holiday
    let holidays = vec![
        Holiday {
            date: "2023-01-01T00:00:00Z".to_string(),
            description: "New Year's Day".to_string(),
        }
    ];

    let _ = test::TestRequest::post()
        .uri("/holidays/fr")
        .set_json(&holidays)
        .send_request(&app)
        .await;

    // List holidays for France
    let resp = test::TestRequest::get()
        .uri("/holidays/fr")
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 200);

    let holidays: Vec<workhours::db::Holiday> = test::read_body_json(resp).await;
    assert_eq!(holidays.len(), 1);
    assert_eq!(holidays[0].date, "2023-01-01T00:00:00Z");
    assert_eq!(holidays[0].description, "New Year's Day");
    assert_eq!(holidays[0].country, "fr");

    // List holidays for a different country (should be empty)
    let resp = test::TestRequest::get()
        .uri("/holidays/de")
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 200);

    let holidays: Vec<workhours::db::Holiday> = test::read_body_json(resp).await;
    assert_eq!(holidays.len(), 0);
}

#[actix_rt::test]
async fn test_get_work_hours_with_end_date() {
    let app = create_test_app().await;

    // Test work hours calculation with end date
    // Monday to Friday, 5 days, 8 hours per day = 40 hours
    let resp = test::TestRequest::get()
        .uri("/?startDate=2023-10-02T09:00:00Z&endDate=2023-10-06T17:00:00Z&country=us&timezone=UTC")
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 200);

    let result: WorkHoursResponse = test::read_body_json(resp).await;
    assert_eq!(result.work_hours, 40.0);
}

#[actix_rt::test]
async fn test_get_work_hours_with_duration() {
    let app = create_test_app().await;

    // Test work hours calculation with duration
    // Starting Monday 9am, duration 5 days
    // Monday to Friday = 5 workdays * 8 hours = 40 hours
    let resp = test::TestRequest::get()
        .uri("/?startDate=2023-10-02T09:00:00Z&durationSeconds=432000&country=us&timezone=UTC")
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), 200);


    let result: WorkHoursResponse = test::read_body_json(resp).await;
    assert_eq!(result.work_hours, 40.0);
}

#[actix_rt::test]
async fn test_get_work_hours_with_holiday() {
    let app = create_test_app().await;

    // Add a holiday
    let holidays = vec![
        Holiday {
            date: "2023-10-04T00:00:00Z".to_string(),
            description: "Test Holiday".to_string(),
        }
    ];

    let _ = test::TestRequest::post()
        .uri("/holidays/us")
        .set_json(&holidays)
        .send_request(&app)
        .await;

    // Test work hours calculation with a holiday
    // Monday to Friday, 5 days, but Wednesday is a holiday
    // So 4 days * 8 hours = 32 hours
    let resp = test::TestRequest::get()
        .uri("/?startDate=2023-10-02T09:00:00Z&endDate=2023-10-06T17:00:00Z&country=us&timezone=UTC")
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 200);

    let result: WorkHoursResponse = test::read_body_json(resp).await;
    assert_eq!(result.work_hours, 32.0);
}

#[actix_rt::test]
async fn test_get_work_hours_invalid_date() {
    let app = create_test_app().await;

    // Test with invalid date format
    let resp = test::TestRequest::get()
        .uri("/?startDate=invalid-date&endDate=2023-10-06T17:00:00Z&country=us&timezone=UTC")
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
async fn test_get_work_hours_invalid_timezone() {
    let app = create_test_app().await;

    // Test with invalid timezone
    let resp = test::TestRequest::get()
        .uri("/?startDate=2023-10-02T09:00:00Z&endDate=2023-10-06T17:00:00Z&country=us&timezone=Invalid/Timezone")
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
async fn test_add_holiday_invalid_country() {
    let app = create_test_app().await;

    // Create a test holiday list
    let holidays = vec![
        Holiday {
            date: "2023-12-25T00:00:00Z".to_string(),
            description: "Christmas".to_string(),
        }
    ];

    // Send POST request with invalid country code
    let resp = test::TestRequest::post()
        .uri("/holidays/invalid")
        .set_json(&holidays)
        .send_request(&app)
        .await;

    // Check response is 400 Bad Request
    assert_eq!(resp.status(), 400);

    // Check error message
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Invalid country code"));
    assert!(body_str.contains("Must be a valid ISO-3166-1 alpha-2 code"));
}

#[actix_rt::test]
async fn test_list_holidays_invalid_country() {
    let app = create_test_app().await;

    // List holidays with invalid country code
    let resp = test::TestRequest::get()
        .uri("/holidays/invalid")
        .send_request(&app)
        .await;

    // Check response is 400 Bad Request
    assert_eq!(resp.status(), 400);

    // Check error message
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Invalid country code"));
    assert!(body_str.contains("Must be a valid ISO-3166-1 alpha-2 code"));
}

#[actix_rt::test]
async fn test_get_work_hours_invalid_country() {
    let app = create_test_app().await;

    // Test with invalid country code
    let resp = test::TestRequest::get()
        .uri("/?startDate=2023-10-02T09:00:00Z&endDate=2023-10-06T17:00:00Z&country=invalid&timezone=UTC")
        .send_request(&app)
        .await;

    // Check response is 400 Bad Request
    assert_eq!(resp.status(), 400);

    // Check error message
    let body = test::read_body(resp).await;
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(body_str.contains("Invalid country code"));
    assert!(body_str.contains("Must be a valid ISO-3166-1 alpha-2 code"));
}
