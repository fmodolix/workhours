use actix_web::{web, HttpResponse, dev::HttpServiceFactory};
use serde_json::json;
use std::fs;
use std::env;

pub fn swagger_spec() -> String {
    // Get server host and port from environment variables or use defaults
    let server_url = env::var("SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    let now = chrono::Utc::now();
    let onehour = (now + chrono::Duration::hours(1)).to_rfc3339();
    let current = now.to_rfc3339();

    serde_json::to_string(&json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Work Hours API",
            "description": "API for calculating work hours between dates, taking into account country-specific holidays and timezones. \n Holidays are taken from the opendata holidays API: https://openholidaysapi.org/swagger/index.html.",
            "version": "1.0.0"
        },
        "servers": [
            {
                "url": server_url,
                "description": "API Server"
            }
        ],
        "paths": {
            "/": {
                "post": {
                    "summary": "Calculate work hours between dates",
                    "description": "Calculates the number of work hours between two dates, taking into account weekends, holidays, and timezones.",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/WorkHoursRequest"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Successful response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/WorkHoursResponse"
                                    }
                                }
                            }
                        },
                        "400": {
                            "description": "Bad request",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "error": {
                                                "type": "string"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
        },
        "components": {
            "schemas": {
                "WorkHoursRequest": {
                    "type": "object",
                    "properties": {
                        "startDate": {
                            "type": "string",
                            "format": "date-time",
                            "default": current
                        },
                        "endDate": {
                            "type": "string",
                            "format": "date-time",
                            "default": onehour

                        },
                        "durationSeconds": {
                            "type": "integer"
                        },
                        "startOfDay": {
                            "type": "string",
                            "format": "time",
                            "default": "09:00:00"
                        },
                        "endOfDay": {
                            "type": "string",
                            "format": "time",
                            "default": "17:00:00"
                        },
                        "country": {
                            "type": "string",
                            "example": "fr",
                            "default": "fr"
                        },
                        "subdivision": {
                            "type": "string",
                            "description": "ISO-3166-2 country subdivision code",
                            "example": "fr",
                            "default": "fr"
                        },
                        "timezone": {
                            "type": "string",
                            "default": "UTC",
                            "example": "Europe/Paris"
                        }
                    }
                },
                "WorkHoursResponse": {
                    "type": "object",
                    "properties": {
                        "workHours": {
                            "type": "number",
                            "format": "float"
                        },
                        "workMinutes": {
                            "type": "number",
                            "format": "float"
                        },
                        "workSeconds": {
                            "type": "number",
                            "format": "float"
                        },
                        "startDate": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "endDate": {
                            "type": "string",
                            "format": "date-time"
                        }
                    }
                }
            }
        }
    })).unwrap()
}

pub async fn serve_swagger_ui() -> HttpResponse {
    match fs::read_to_string("swagger-ui.html") {
        Ok(content) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(content),
        Err(_) => HttpResponse::InternalServerError()
            .body("Could not read swagger-ui.html file")
    }
}

pub async fn serve_swagger_schema() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(swagger_spec())
}

pub fn swagger_routes() -> impl HttpServiceFactory {
    web::scope("")
        .service(web::resource("/").route(web::get().to(serve_swagger_ui)))
        .service(web::resource("/schema").route(web::get().to(serve_swagger_schema)))
}
