//! Error types and HTTP error responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// JSON error response body.
#[derive(Serialize)]
struct ErrorBody {
    error: String,
    detail: String,
}

/// API-level errors (database, auth, not found, etc.).
#[derive(Debug)]
pub enum ApiError {
    /// Calculation engine error.
    Calculation(isso51_core::error::Isso51Error),
    /// Database error.
    Database(String),
    /// Invalid client input (e.g. malformed path parameter).
    BadRequest(String),
    /// Resource not found.
    NotFound(String),
    /// Forbidden (ownership check failed).
    Forbidden(String),
    /// Internal server error.
    Internal(String),
    /// External report service error.
    ReportService(String),
    /// De Reports API keurde het rapport af (4xx). Status en `detail` gaan
    /// ongewijzigd door, zodat de gebruiker de oorzaak leest.
    ReportRejected {
        status: StatusCode,
        detail: String,
    },
    /// Service not configured / unavailable.
    ServiceUnavailable(String),
    /// Constructie-rekenkern: invoerfout of niet beschikbaar (HTTP 422).
    Constructie(constructie_core::ConstructieFout),
    /// Module niet geactiveerd voor dit account (HTTP 403). De tekst is de
    /// letterlijke `error`-waarde in de response.
    ModuleNietGeactiveerd(String),
}

impl From<constructie_core::ConstructieFout> for ApiError {
    fn from(err: constructie_core::ConstructieFout) -> Self {
        ApiError::Constructie(err)
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("Database error: {err}");
        ApiError::Database("Database error".to_string())
    }
}

impl From<isso51_core::error::Isso51Error> for ApiError {
    fn from(err: isso51_core::error::Isso51Error) -> Self {
        ApiError::Calculation(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, detail) = match self {
            ApiError::Calculation(err) => {
                return into_calc_response(err);
            }
            ApiError::Database(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database_error", msg)
            }
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
            ApiError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg)
            }
            ApiError::ReportService(msg) => {
                (StatusCode::BAD_GATEWAY, "report_service_error", msg)
            }
            ApiError::ReportRejected { status, detail } => (status, "report_rejected", detail),
            ApiError::ServiceUnavailable(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable", msg)
            }
            ApiError::Constructie(err) => {
                use constructie_core::ConstructieFout;
                match &err {
                    ConstructieFout::Json(_) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "json_error", err.to_string())
                    }
                    ConstructieFout::Invoer(_) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "invalid_input", err.to_string())
                    }
                    ConstructieFout::NietBeschikbaar(_) => {
                        (StatusCode::UNPROCESSABLE_ENTITY, "not_available", err.to_string())
                    }
                    ConstructieFout::Data(_) => {
                        tracing::error!("Constructie-profieldata ongeldig: {err}");
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "internal_error",
                            "interne fout in de rekenkern".to_string(),
                        )
                    }
                }
            }
            ApiError::ModuleNietGeactiveerd(msg) => {
                let body = serde_json::json!({ "error": msg });
                return (StatusCode::FORBIDDEN, axum::Json(body)).into_response();
            }
        };

        let body = ErrorBody {
            error: error_type.to_string(),
            detail,
        };

        (status, axum::Json(body)).into_response()
    }
}

/// Map `isso51_ifcx::IfcxError` to an HTTP response.
pub fn into_ifcx_response(err: isso51_ifcx::IfcxError) -> Response {
    use isso51_ifcx::IfcxError;

    let (status, error_type, detail) = match err {
        IfcxError::MissingEntry(entry) => (
            StatusCode::BAD_REQUEST,
            "missing_entry",
            format!("Missing required IFC entry: {entry}"),
        ),
        IfcxError::MissingAttribute(entry, attr) => (
            StatusCode::BAD_REQUEST,
            "missing_attribute",
            format!("Missing required attribute '{attr}' on {entry}"),
        ),
        IfcxError::Json(e) => (
            StatusCode::BAD_REQUEST,
            "json_error",
            e.to_string(),
        ),
        IfcxError::Calc(calc_err) => {
            return into_calc_response(calc_err);
        }
    };

    let body = ErrorBody {
        error: error_type.to_string(),
        detail,
    };

    (status, axum::Json(body)).into_response()
}

/// Map `isso51_core::error::Isso51Error` to an HTTP response.
pub fn into_calc_response(err: isso51_core::error::Isso51Error) -> Response {
    use isso51_core::error::Isso51Error;

    let (status, error_type) = match &err {
        Isso51Error::InvalidInput(_) => (StatusCode::BAD_REQUEST, "invalid_input"),
        Isso51Error::Json(_) => (StatusCode::BAD_REQUEST, "json_error"),
        Isso51Error::MissingParameter(_) => (StatusCode::BAD_REQUEST, "missing_parameter"),
        Isso51Error::RoomNotFound(_) => (StatusCode::NOT_FOUND, "room_not_found"),
        Isso51Error::OutOfRange { .. } => {
            (StatusCode::UNPROCESSABLE_ENTITY, "out_of_range")
        }
        Isso51Error::InfiltrationConfig(_) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "infiltration_config")
        }
        Isso51Error::VabiImport(_) => (StatusCode::BAD_REQUEST, "vabi_import"),
        Isso51Error::VabiZipError(_) => (StatusCode::BAD_REQUEST, "vabi_zip_error"),
        Isso51Error::VabiSqliteError(_) => (StatusCode::BAD_REQUEST, "vabi_sqlite_error"),
    };

    let body = ErrorBody {
        error: error_type.to_string(),
        detail: err.to_string(),
    };

    (status, axum::Json(body)).into_response()
}
