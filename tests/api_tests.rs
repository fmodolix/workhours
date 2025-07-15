use actix_web::{test, web, App};
use actix_http;
use workhours::{get_work_hours, WorkHoursQueryParams, WorkHoursResponse};

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
            .service(get_work_hours)
    ).await
}

#[actix_rt::test]
async fn test_get_work_hours_with_end_date() {
    let app = create_test_app().await;
    let red = WorkHoursQueryParams {
        start_date: "2023-10-02T09:00:00Z".to_string(),
        end_date: Some("2023-10-06T17:00:00Z".to_string()),
        start_of_day: "09:00:00".to_string(),
        end_of_day: "17:00:00".to_string(),
        duration_seconds: None,
        country: "us".to_string(),
        timezone: "UTC".to_string(),
        subdivision: None
    };

    // Test work hours calculation with end date
    // Monday to Friday, 5 days, 8 hours per day = 40 hours
    let resp = test::TestRequest::post()
        .uri("/")
        .set_json(red)
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 200);

    let result: WorkHoursResponse = test::read_body_json(resp).await;
    assert_eq!(result.work_hours, 40.0);
}

#[actix_rt::test]
async fn test_get_work_hours_with_duration() {
    let app = create_test_app().await;
    let red = WorkHoursQueryParams {
        start_date: "2023-10-02T09:00:00Z".to_string(),
        end_date: None,
        start_of_day: "09:00:00".to_string(),
        end_of_day: "17:00:00".to_string(),
        duration_seconds: Some(432000),
        country: "us".to_string(),
        timezone: "UTC".to_string(),
        subdivision: None
    };
    // Test work hours calculation with duration
    // Starting Monday 9am, duration 5 days
    // Monday to Friday = 5 workdays * 8 hours = 40 hours
    let resp = test::TestRequest::post()
        .uri("/")
        .set_json(red)
        .send_request(&app)
        .await;
    assert_eq!(resp.status(), 200);


    let result: WorkHoursResponse = test::read_body_json(resp).await;
    assert_eq!(result.work_hours, 40.0);
}

#[actix_rt::test]
async fn test_get_work_hours_invalid_date() {
    let app = create_test_app().await;
    let red = WorkHoursQueryParams {
        start_date: "invalid-date".to_string(),
        end_date: Some("2023-10-06T17:00:00Z".to_string()),
        start_of_day: "09:00:00".to_string(),
        end_of_day: "17:00:00".to_string(),
        duration_seconds: None,
        country: "us".to_string(),
        timezone: "UTC".to_string(),
        subdivision: None
    };

    // Test with invalid date format
    let resp = test::TestRequest::post()
        .uri("/")
        .set_json(red)
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 400);
}

#[actix_rt::test]
async fn test_get_work_hours_invalid_timezone() {
    let app = create_test_app().await;
    let red = WorkHoursQueryParams {
        start_date: "2023-10-02T09:00:00Z".to_string(),
        end_date: Some("2023-10-06T17:00:00Z".to_string()),
        start_of_day: "09:00:00".to_string(),
        end_of_day: "17:00:00".to_string(),
        duration_seconds: None,
        country: "us".to_string(),
        timezone: "Invalid/Timezone".to_string(),
        subdivision: None
    };

    // Test with invalid timezone
    let resp = test::TestRequest::post()
        .uri("/")
        .set_json(red)
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 400);
}
