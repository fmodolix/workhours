use actix_web::{web, HttpResponse, dev::HttpServiceFactory};
use serde_json::json;
use std::fs;
use std::env;

pub fn swagger_spec() -> String {
    // Get server host and port from environment variables or use defaults
    let server_url = env::var("SERVER_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    serde_json::to_string(&json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Work Hours API",
            "description": "API for calculating work hours between dates, taking into account country-specific holidays and timezones",
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
                "get": {
                    "summary": "Calculate work hours between dates",
                    "description": "Calculates the number of work hours between two dates, taking into account weekends, holidays, and timezones",
                    "parameters": [
                        {
                            "in": "query",
                            "name": "startDate",
                            "required": true,
                            "schema": {
                                "type": "string",
                                "format": "date-time"
                            },
                            "description": "Start date in RFC3339 format"
                        },
                        {
                            "in": "query",
                            "name": "endDate",
                            "required": false,
                            "schema": {
                                "type": "string",
                                "format": "date-time"
                            },
                            "description": "End date in RFC3339 format (use either endDate or durationSeconds)"
                        },
                        {
                            "in": "query",
                            "name": "durationSeconds",
                            "required": false,
                            "schema": {
                                "type": "integer"
                            },
                            "description": "Duration in seconds (use either endDate or durationSeconds)"
                        },
                        {
                            "in": "query",
                            "name": "startOfDay",
                            "required": false,
                            "schema": {
                                "type": "string",
                                "format": "time"
                            },
                            "description": "Time to start counting work hours from (e.g. \"07:00:00\")",
                            "default": "09:00:00"
                        },
                        {
                            "in": "query",
                            "name": "EndOfDay",
                            "required": false,
                            "schema": {
                                "type": "string",
                                "format": "time"
                            },
                            "description": "Time to stop counting work hours from (e.g. \"18:00:00\")",
                            "default": "17:00:00"
                        },
                        {
                            "in": "query",
                            "name": "country",
                            "required": true,
                            "schema": {
                                "type": "string"
                            },
                            "description": "Country code (e.g. \"fr\" for France)",
                            "default": "fr"
                        },
                        {
                            "in": "query",
                            "name": "timezone",
                            "required": true,
                            "schema": {
                                "type": "string"
                            },
                            "description": "Timezone in IANA format (e.g. \"Europe/Paris\")",
                            "default": "Europe/Paris"

                        }
                    ],
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
            "/holidays/{country}": {
                "post": {
                    "summary": "Add holidays for a country",
                    "description": "Adds multiple holidays for the specified country",
                    "parameters": [
                        {
                            "in": "path",
                            "name": "country",
                            "required": true,
                            "schema": {
                                "type": "string"
                            },
                            "description": "Country code"
                        }
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "array",
                                    "items": {
                                        "$ref": "#/components/schemas/Holiday"
                                    }
                                }
                            }
                        }
                    },
                    "responses": {
                        "201": {
                            "description": "Holiday added successfully",
                            "content": {
                                "text/plain": {
                                    "schema": {
                                        "type": "string"
                                    }
                                }
                            }
                        }
                    }
                },
                "get": {
                    "summary": "List holidays for a country",
                    "description": "Adds a holiday for the specified country",
                    "parameters": [
                        {
                            "in": "path",
                            "name": "country",
                            "required": true,
                            "schema": {
                                "type": "string"
                            },
                            "description": "Country code"
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "List of holidays for the specified country",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "array",
                                        "items": {
                                            "$ref": "#/components/schemas/Holiday"
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
                            "format": "date-time"
                        },
                        "endDate": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "durationSeconds": {
                            "type": "integer"
                        },
                        "startOfDay": {
                            "type": "time"
                        },
                        "endOfDay": {
                            "type": "time"
                        },
                        "country": {
                            "type": "string"
                        },
                        "timezone": {
                            "type": "string"
                        }
                    }
                },
                "Holiday": {
                    "type": "object",
                    "required": ["date"],
                    "properties": {
                        "date": {
                            "type": "string",
                            "format": "date-time"
                        },
                        "description": {
                            "type": "string"
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
        .service(web::resource("/swagger").route(web::get().to(serve_swagger_ui)))
        .service(web::resource("/schema").route(web::get().to(serve_swagger_schema)))
}
