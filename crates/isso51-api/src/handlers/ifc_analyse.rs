//! IFC-analyse handler: pure doorgifte naar de private sidecar-service
//! (`ifc-ruimtebalans`). Geen engine-code in deze crate.
//!
//! De upload wordt als multipart doorgezet naar `{IFC_ANALYSE_URL}/analyse`;
//! het JSON-resultaat gaat terug naar de client, aangevuld met
//! `source_filename`.

use std::time::Duration;

use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::{Map, Value};

use super::ifc_import::safe_upload_filename_in;
use crate::auth::AuthClaims;
use crate::state::AppState;

/// Maximum upload size: 100 MB.
const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;

/// Toegestane upload-extensies voor de analyse (lowercase).
const ALLOWED_ANALYSE_EXTENSIONS: &[&str] = &["ifc", "ifczip"];

/// Pad van het analyse-endpoint op de sidecar.
const SIDECAR_ANALYSE_PATH: &str = "/analyse";

/// Naam van het multipart-veld (zowel inkomend als richting sidecar).
const FILE_FIELD: &str = "file";

/// Veldnaam waarin wij de originele clientbestandsnaam teruggeven.
const SOURCE_FILENAME_FIELD: &str = "source_filename";

/// Maximale lengte (tekens) van `source_filename`.
const MAX_SOURCE_FILENAME_CHARS: usize = 255;

/// Maximale lengte (tekens) van een doorgegeven sidecar-foutmelding.
const MAX_DETAIL_CHARS: usize = 500;

/// Fallback-bestandsnaam als de client er geen meestuurt.
const DEFAULT_CLIENT_FILENAME: &str = "upload.ifc";

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    detail: String,
}

fn error_response(status: StatusCode, error: &str, detail: impl Into<String>) -> Response {
    let body = ErrorBody {
        error: error.to_string(),
        detail: detail.into(),
    };
    (status, Json(body)).into_response()
}

fn unavailable() -> Response {
    error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "analyse_unavailable",
        "IFC-analyse niet beschikbaar",
    )
}

/// `POST /ifc/analyse` — Geef een IFC-upload door aan de analyse-sidecar.
///
/// Vereist een geauthenticeerde caller. Statuscodes: 200 (resultaat),
/// 400 (ontbrekend bestand / extensie), 413 (te groot), 422 (sidecar wijst
/// bestand af), 502 (sidecar-fout), 503 (niet beschikbaar), 504 (timeout).
///
/// NB: hier komt later de module-gating (entitlement-middleware); nu alleen
/// authenticatie.
pub async fn analyse_ifc(
    State(state): State<AppState>,
    AuthClaims(_claims): AuthClaims,
    mut multipart: Multipart,
) -> Response {
    // Eerst de configuratie checken, nog vóór de body gelezen wordt.
    let Some(base_url) = state.ifc_analyse_url.as_deref() else {
        return unavailable();
    };

    let mut file_data: Option<(String, bytes::Bytes)> = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => return upload_error(&e),
        };
        if field.name() != Some(FILE_FIELD) {
            continue;
        }
        let filename = field
            .file_name()
            .unwrap_or(DEFAULT_CLIENT_FILENAME)
            .to_string();
        match field.bytes().await {
            Ok(bytes) => file_data = Some((filename, bytes)),
            Err(e) => return upload_error(&e),
        }
        break;
    }

    let Some((filename, data)) = file_data else {
        return error_response(
            StatusCode::BAD_REQUEST,
            "missing_file",
            "Geen 'file' veld in multipart upload",
        );
    };

    if data.len() > MAX_FILE_SIZE {
        return too_large(data.len());
    }

    let safe_name = match safe_upload_filename_in(&filename, ALLOWED_ANALYSE_EXTENSIONS) {
        Ok(n) => n,
        Err(detail) => return error_response(StatusCode::BAD_REQUEST, "invalid_extension", detail),
    };

    forward_to_sidecar(
        &state.http_client,
        base_url,
        state.ifc_analyse_timeout,
        &sanitize_source_filename(&filename),
        &safe_name,
        data,
    )
    .await
}

fn too_large(len: usize) -> Response {
    error_response(
        StatusCode::PAYLOAD_TOO_LARGE,
        "file_too_large",
        format!(
            "Bestand is te groot ({:.1} MB, max {} MB)",
            len as f64 / 1_048_576.0,
            MAX_FILE_SIZE / 1_048_576
        ),
    )
}

fn upload_error(e: &axum::extract::multipart::MultipartError) -> Response {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return error_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            "file_too_large",
            format!("Bestand is te groot (max {} MB)", MAX_FILE_SIZE / 1_048_576),
        );
    }
    error_response(
        StatusCode::BAD_REQUEST,
        "upload_error",
        format!("Bestand lezen mislukt: {e}"),
    )
}

/// Reduceer een client-bestandsnaam tot de basename (zonder pad), max
/// [`MAX_SOURCE_FILENAME_CHARS`] tekens.
fn sanitize_source_filename(client_filename: &str) -> String {
    client_filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_SOURCE_FILENAME_CHARS)
        .collect()
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Geef de bytes door aan `{base_url}/analyse` en vertaal het antwoord.
///
/// Losgekoppeld van de extractors zodat het zonder auth te testen is.
/// `source_filename` wordt in het resultaat gezet (overschrijft sidecar-veld);
/// `upload_name` is de veilige bestandsnaam richting sidecar.
async fn forward_to_sidecar(
    client: &reqwest::Client,
    base_url: &str,
    timeout: Duration,
    source_filename: &str,
    upload_name: &str,
    data: bytes::Bytes,
) -> Response {
    let url = format!("{}{SIDECAR_ANALYSE_PATH}", base_url.trim_end_matches('/'));
    let size = data.len();

    let part = reqwest::multipart::Part::stream(data).file_name(upload_name.to_string());
    let form = reqwest::multipart::Form::new().part(FILE_FIELD, part);

    tracing::info!(size_bytes = size, "IFC-analyse: doorgifte naar sidecar");

    let result = async {
        let resp = client
            .post(&url)
            .multipart(form)
            .timeout(timeout)
            .send()
            .await?;
        let status = resp.status();
        let body = resp.bytes().await?;
        Ok::<_, reqwest::Error>((status, body))
    }
    .await;

    let (status, body) = match result {
        Ok(r) => r,
        Err(e) if e.is_timeout() => {
            tracing::error!("IFC-analyse: sidecar timeout na {timeout:?}");
            return error_response(
                StatusCode::GATEWAY_TIMEOUT,
                "analyse_timeout",
                format!(
                    "IFC-analyse duurde te lang (max {} seconden)",
                    timeout.as_secs()
                ),
            );
        }
        Err(e) => {
            tracing::error!(
                is_connect = e.is_connect(),
                "IFC-analyse: sidecar niet bereikbaar: {e}"
            );
            return unavailable();
        }
    };

    let json: Option<Value> = serde_json::from_slice(&body).ok();

    if status.is_success() {
        return match json {
            Some(Value::Object(mut map)) => {
                map.insert(
                    SOURCE_FILENAME_FIELD.to_string(),
                    Value::String(source_filename.to_string()),
                );
                Json(Value::Object(map)).into_response()
            }
            _ => {
                tracing::error!(%status, "IFC-analyse: sidecar gaf geen JSON-object");
                bad_gateway("Sidecar gaf geen geldig JSON-resultaat")
            }
        };
    }

    if status.is_client_error() {
        let detail = json.as_ref().and_then(extract_detail).unwrap_or_else(|| {
            truncate_chars(String::from_utf8_lossy(&body).trim(), MAX_DETAIL_CHARS)
        });
        tracing::warn!(%status, "IFC-analyse: sidecar wijst upload af");
        return error_response(StatusCode::UNPROCESSABLE_ENTITY, "analyse_failed", detail);
    }

    tracing::error!(%status, "IFC-analyse: sidecar-fout");
    bad_gateway(&format!("Sidecar gaf status {}", status.as_u16()))
}

/// Haal `detail` (anders `error`) als tekst uit een sidecar-foutobject.
fn extract_detail(v: &Value) -> Option<String> {
    let map: &Map<String, Value> = v.as_object()?;
    ["detail", "error"].iter().find_map(|k| match map.get(*k)? {
        Value::String(s) => Some(truncate_chars(s, MAX_DETAIL_CHARS)),
        Value::Null => None,
        other => Some(truncate_chars(&other.to_string(), MAX_DETAIL_CHARS)),
    })
}

fn bad_gateway(detail: &str) -> Response {
    error_response(StatusCode::BAD_GATEWAY, "analyse_error", detail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::post;
    use axum::Router;
    use serde_json::json;

    const TEST_TIMEOUT: Duration = Duration::from_secs(5);

    /// Start een nep-sidecar op een vrije poort en geef de basis-URL terug.
    async fn spawn_sidecar(router: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        format!("http://{addr}")
    }

    async fn call(base: &str, timeout: Duration) -> (StatusCode, Value) {
        let resp = forward_to_sidecar(
            &reqwest::Client::new(),
            base,
            timeout,
            "model.ifc",
            "upload.ifc",
            bytes::Bytes::from_static(b"ISO-10303-21;"),
        )
        .await;
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }

    #[tokio::test]
    async fn doorgifte_200_injecteert_source_filename() {
        let app = Router::new().route(
            "/analyse",
            post(|mut mp: Multipart| async move {
                let f = mp.next_field().await.unwrap().unwrap();
                assert_eq!(f.name(), Some("file"));
                assert_eq!(f.file_name(), Some("upload.ifc"));
                let n = f.bytes().await.unwrap().len();
                Json(json!({"rooms": 3, "bytes": n, "source_filename": "sidecar.ifc"}))
            }),
        );
        let base = spawn_sidecar(app).await;
        // Trailing slash moet getrimd worden.
        let (status, body) = call(&format!("{base}/"), TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["rooms"], 3);
        assert_eq!(body["bytes"], 13);
        assert_eq!(body["source_filename"], "model.ifc");
    }

    #[tokio::test]
    async fn sidecar_4xx_wordt_422_met_detail() {
        let app = Router::new().route(
            "/analyse",
            post(|| async {
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"detail": "geen IfcSpace"})),
                )
            }),
        );
        let (status, body) = call(&spawn_sidecar(app).await, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "analyse_failed");
        assert_eq!(body["detail"], "geen IfcSpace");
    }

    #[tokio::test]
    async fn sidecar_4xx_platte_tekst_wordt_ingekort() {
        let app = Router::new().route(
            "/analyse",
            post(|| async { (StatusCode::UNPROCESSABLE_ENTITY, "x".repeat(900)) }),
        );
        let (status, body) = call(&spawn_sidecar(app).await, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["detail"].as_str().unwrap().len(), MAX_DETAIL_CHARS);
    }

    #[tokio::test]
    async fn sidecar_5xx_wordt_502() {
        let app = Router::new().route(
            "/analyse",
            post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "boem") }),
        );
        let (status, body) = call(&spawn_sidecar(app).await, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["error"], "analyse_error");
    }

    #[tokio::test]
    async fn sidecar_2xx_zonder_json_object_wordt_502() {
        let app = Router::new().route("/analyse", post(|| async { Json(json!([1, 2])) }));
        let (status, body) = call(&spawn_sidecar(app).await, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["error"], "analyse_error");
    }

    #[tokio::test]
    async fn timeout_wordt_504() {
        let app = Router::new().route(
            "/analyse",
            post(|| async {
                tokio::time::sleep(Duration::from_secs(3)).await;
                Json(json!({}))
            }),
        );
        let (status, body) = call(&spawn_sidecar(app).await, Duration::from_millis(200)).await;
        assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(body["error"], "analyse_timeout");
    }

    #[tokio::test]
    async fn verbinding_geweigerd_wordt_503() {
        // Bind, noteer poort, sluit weer: niemand luistert meer.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);
        let (status, body) = call(&format!("http://{addr}"), TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["error"], "analyse_unavailable");
    }

    #[test]
    fn unavailable_heeft_vaste_melding() {
        let resp = unavailable();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn source_filename_wordt_basename_en_afgekapt() {
        assert_eq!(sanitize_source_filename("../../etc/model.ifc"), "model.ifc");
        assert_eq!(sanitize_source_filename("C:\\tmp\\a b.ifc"), "a b.ifc");
        let lang = format!("{}.ifc", "a".repeat(400));
        assert_eq!(
            sanitize_source_filename(&lang).chars().count(),
            MAX_SOURCE_FILENAME_CHARS
        );
    }
}
