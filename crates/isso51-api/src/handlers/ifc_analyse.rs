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

/// Standaard `Retry-After` (seconden) als de sidecar bij BEZET geen header meestuurt.
const DEFAULT_RETRY_AFTER_S: u64 = 30;

/// Sidecar-foutcode bij een lopende analyse (HTTP 503).
const SIDECAR_BUSY_CODE: &str = "BEZET";

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
        state.ifc_analyse_token.as_deref(),
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
/// `upload_name` is de veilige bestandsnaam richting sidecar. Is `token`
/// gezet, dan gaat het als `Authorization: Bearer` mee (nooit gelogd).
///
/// Er worden bewust GEEN extra multipart-velden meegestuurd (`profile`,
/// `surfaces_ifc`, `exported_at`): de sidecar hanteert dan zijn eigen
/// defaults (profiel `kba`, geen vlakken-IFC).
///
/// Timeout: de caller geeft de waarde uit `IFC_ANALYSE_TIMEOUT_S` (default
/// 300 s). Ter referentie: model 2786 duurt ca. 46 s lokaal; de harde grens
/// van de sidecar zelf is 600 s.
///
/// Foutmapping sidecar naar API: 400/422 naar 422 `analyse_failed`; 413 naar
/// 413 `file_too_large`; 503 `BEZET` naar 503 `analyse_busy` (+ `Retry-After`);
/// 504 naar 504 `analyse_timeout`; 401, 500 en overige statussen naar 502
/// `analyse_error`. `stderr` van de sidecar gaat nooit naar de client.
async fn forward_to_sidecar(
    client: &reqwest::Client,
    base_url: &str,
    timeout: Duration,
    source_filename: &str,
    upload_name: &str,
    data: bytes::Bytes,
    token: Option<&str>,
) -> Response {
    let url = format!("{}{SIDECAR_ANALYSE_PATH}", base_url.trim_end_matches('/'));
    let size = data.len();

    let part = reqwest::multipart::Part::stream(data).file_name(upload_name.to_string());
    let form = reqwest::multipart::Form::new().part(FILE_FIELD, part);

    tracing::info!(size_bytes = size, "IFC-analyse: doorgifte naar sidecar");

    let mut request = client.post(&url).multipart(form).timeout(timeout);
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }

    let result = async {
        let resp = request.send().await?;
        let status = resp.status();
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok());
        let body = resp.bytes().await?;
        Ok::<_, reqwest::Error>((status, retry_after, body))
    }
    .await;

    let (status, retry_after, body) = match result {
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
            let is_connect = e.is_connect();
            let melding = e.without_url().to_string();
            tracing::error!(
                is_connect,
                "IFC-analyse: sidecar niet bereikbaar: {melding}"
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

    // Detail uit het sidecar-foutobject, anders de body-tekst; altijd ingekort.
    let detail = json
        .as_ref()
        .and_then(extract_detail)
        .unwrap_or_else(|| truncate_chars(String::from_utf8_lossy(&body).trim(), MAX_DETAIL_CHARS));
    let code = json
        .as_ref()
        .and_then(|v| v.get("error"))
        .and_then(Value::as_str)
        .unwrap_or("");

    match status.as_u16() {
        400 | 422 => {
            tracing::warn!(%status, "IFC-analyse: sidecar wijst upload af");
            error_response(StatusCode::UNPROCESSABLE_ENTITY, "analyse_failed", detail)
        }
        413 => {
            tracing::warn!("IFC-analyse: sidecar vindt bestand te groot");
            error_response(StatusCode::PAYLOAD_TOO_LARGE, "file_too_large", detail)
        }
        503 if code == SIDECAR_BUSY_CODE => {
            tracing::warn!("IFC-analyse: sidecar bezet");
            let mut resp = error_response(StatusCode::SERVICE_UNAVAILABLE, "analyse_busy", detail);
            resp.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                retry_after_value(retry_after),
            );
            resp
        }
        504 => {
            tracing::error!("IFC-analyse: sidecar meldt timeout");
            error_response(StatusCode::GATEWAY_TIMEOUT, "analyse_timeout", detail)
        }
        other => {
            // 500 bevat `exit_code` en `stderr`: alleen ingekort loggen, nooit doorgeven.
            let stderr = json
                .as_ref()
                .and_then(|v| v.get("stderr"))
                .and_then(Value::as_str)
                .map(|s| truncate_chars(s, MAX_DETAIL_CHARS))
                .unwrap_or_default();
            tracing::error!(status = other, stderr = %stderr, "IFC-analyse: sidecar-fout");
            bad_gateway(&detail)
        }
    }
}

/// `Retry-After`-waarde: die van de sidecar, anders [`DEFAULT_RETRY_AFTER_S`].
fn retry_after_value(seconds: Option<u64>) -> axum::http::HeaderValue {
    axum::http::HeaderValue::from(seconds.unwrap_or(DEFAULT_RETRY_AFTER_S))
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
        let (status, _, body) = call_full(base, timeout, None).await;
        (status, body)
    }

    async fn call_full(
        base: &str,
        timeout: Duration,
        token: Option<&str>,
    ) -> (StatusCode, axum::http::HeaderMap, Value) {
        let resp = forward_to_sidecar(
            &reqwest::Client::new(),
            base,
            timeout,
            "model.ifc",
            "upload.ifc",
            bytes::Bytes::from_static(b"ISO-10303-21;"),
            token,
        )
        .await;
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (
            status,
            headers,
            serde_json::from_slice(&body).unwrap_or(Value::Null),
        )
    }

    /// Nep-sidecar die altijd met `status` en JSON-body antwoordt.
    async fn sidecar_with(
        status: StatusCode,
        body: Value,
        retry_after: Option<&'static str>,
    ) -> String {
        let app = Router::new().route(
            "/analyse",
            post(move || {
                let body = body.clone();
                async move {
                    let mut resp = (status, Json(body)).into_response();
                    if let Some(v) = retry_after {
                        resp.headers_mut()
                            .insert("retry-after", axum::http::HeaderValue::from_static(v));
                    }
                    resp
                }
            }),
        );
        spawn_sidecar(app).await
    }

    #[tokio::test]
    async fn bezet_wordt_503_analyse_busy_met_retry_after() {
        let base = sidecar_with(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"error": "BEZET", "detail": "Er loopt al een analyse"}),
            Some("12"),
        )
        .await;
        let (status, headers, body) = call_full(&base, TEST_TIMEOUT, None).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["error"], "analyse_busy");
        assert_eq!(body["detail"], "Er loopt al een analyse");
        assert_eq!(headers["retry-after"], "12");
    }

    #[tokio::test]
    async fn bezet_zonder_retry_after_krijgt_default_30() {
        let base = sidecar_with(
            StatusCode::SERVICE_UNAVAILABLE,
            json!({"error": "BEZET", "detail": "bezet"}),
            None,
        )
        .await;
        let (_, headers, _) = call_full(&base, TEST_TIMEOUT, None).await;
        assert_eq!(headers["retry-after"], "30");
    }

    #[tokio::test]
    async fn sidecar_413_wordt_413_file_too_large() {
        let base = sidecar_with(
            StatusCode::PAYLOAD_TOO_LARGE,
            json!({"error": "TE_GROOT", "detail": "Boven 200 MB"}),
            None,
        )
        .await;
        let (status, body) = call(&base, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(body["error"], "file_too_large");
        assert_eq!(body["detail"], "Boven 200 MB");
    }

    #[tokio::test]
    async fn sidecar_504_wordt_504_analyse_timeout() {
        let base = sidecar_with(
            StatusCode::GATEWAY_TIMEOUT,
            json!({"error": "TIME_OUT", "detail": "Boven 600 s"}),
            None,
        )
        .await;
        let (status, body) = call(&base, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(body["error"], "analyse_timeout");
    }

    #[tokio::test]
    async fn sidecar_422_ifc_onleesbaar_wordt_422() {
        let base = sidecar_with(
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({"error": "IFC_ONLEESBAAR", "detail": "Kan IFC niet lezen"}),
            None,
        )
        .await;
        let (status, body) = call(&base, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "analyse_failed");
        assert_eq!(body["detail"], "Kan IFC niet lezen");
    }

    #[tokio::test]
    async fn sidecar_401_wordt_502() {
        let base = sidecar_with(
            StatusCode::UNAUTHORIZED,
            json!({"error": "GEEN_TOEGANG", "detail": "Token ontbreekt"}),
            None,
        )
        .await;
        let (status, body) = call(&base, TEST_TIMEOUT).await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["error"], "analyse_error");
    }

    #[tokio::test]
    async fn sidecar_500_lekt_geen_stderr_of_exit_code() {
        let base = sidecar_with(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "error": "ENGINE_FOUT",
                "detail": "Engine stopte",
                "exit_code": 3,
                "stderr": "Traceback GEHEIM-PAD C:\\intern"
            }),
            None,
        )
        .await;
        let resp = forward_to_sidecar(
            &reqwest::Client::new(),
            &base,
            TEST_TIMEOUT,
            "m.ifc",
            "upload.ifc",
            bytes::Bytes::from_static(b"x"),
            None,
        )
        .await;
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
        let raw = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let tekst = String::from_utf8_lossy(&raw);
        assert!(tekst.contains("analyse_error"));
        assert!(tekst.contains("Engine stopte"));
        assert!(!tekst.contains("GEHEIM"));
        assert!(!tekst.contains("stderr"));
        assert!(!tekst.contains("exit_code"));
    }

    #[tokio::test]
    async fn token_wordt_als_bearer_meegestuurd() {
        let app = Router::new().route(
            "/analyse",
            post(|headers: axum::http::HeaderMap| async move {
                let auth = headers
                    .get("authorization")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                Json(json!({"auth": auth}))
            }),
        );
        let base = spawn_sidecar(app).await;
        let (_, _, met) = call_full(&base, TEST_TIMEOUT, Some("geheim123")).await;
        assert_eq!(met["auth"], "Bearer geheim123");
        let (_, _, zonder) = call_full(&base, TEST_TIMEOUT, None).await;
        assert_eq!(zonder["auth"], "");
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
