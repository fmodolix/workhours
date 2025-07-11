use actix_web::web;
use serde_json::json;

pub fn swagger_spec() -> String {
    serde_json::to_string(&json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Work Hours API",
            "description": "API for calculating work hours between dates, taking into account country-specific holidays and timezones",
            "version": "1.0.0"
        },
        "servers": [
            {
                "url": "http://localhost:8080",
                "description": "Local server"
            }
        ],
        "paths": {
            "/workhours": {
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
                            "name": "country",
                            "required": true,
                            "schema": {
                                "type": "string"
                            },
                            "description": "Country code (e.g. \"fr\" for France)"
                        },
                        {
                            "in": "query",
                            "name": "timezone",
                            "required": true,
                            "schema": {
                                "type": "string"
                            },
                            "description": "Timezone in IANA format (e.g. \"Europe/Paris\")"
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
                    "summary": "Add a holiday for a country",
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
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": "#/components/schemas/Holiday"
                                }
                            }
                        }
                    },
                    "responses": {
                        "200": {
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
                }
            }
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

pub fn swagger_route() -> impl actix_web::dev::HttpServiceFactory {
    web::scope("/swagger")
        .route("", web::get().to(|| async { swagger_spec() }))
}
