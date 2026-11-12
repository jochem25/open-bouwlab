//! Trapontwerper (O-2026-10-30-14): invoer naar toetsing, plattegrond en IFC.
//!
//! Alle routes vereisen een ingelogde gebruiker (`AuthClaims`, 401 zonder
//! login) en het entitlement `trap` (403 zonder), zie [`crate::entitlements`];
//! zelfde patroon als de constructiemodule (besluit Jochem 12-11-2026,
//! O-2026-11-12-01). De rekenkern staat in `trap-core`; dit bestand doet alleen
//! HTTP.

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;
use trap_core::{ontwerp_uitvoer, Fout, Invoer, Uitvoer};

use crate::auth::{AuthClaims, OidcClaims};
use crate::entitlements::trap_toegestaan;
use crate::error::ApiError;
use crate::state::AppState;

/// Foutmelding bij ontbrekend entitlement (letterlijk in de response).
const GEEN_TOEGANG: &str = "module trap niet geactiveerd voor dit account";

fn fout(status: StatusCode, code: &str, detail: String) -> Response {
    (status, Json(serde_json::json!({ "error": code, "detail": detail }))).into_response()
}

/// Routes van de trapontwerper (relatief aan het API-prefix).
pub fn trap_routes() -> Router<AppState> {
    Router::new()
        .route("/trap/ontwerp", post(trap_ontwerp))
        .route("/trap/schema", get(trap_schema))
}

/// Controleer het entitlement; anders 403.
fn toegang(state: &AppState, claims: &OidcClaims) -> Result<(), ApiError> {
    if trap_toegestaan(claims, &state.entitlements) {
        Ok(())
    } else {
        Err(ApiError::ModuleNietGeactiveerd(GEEN_TOEGANG.to_string()))
    }
}

/// POST /trap/ontwerp: invoer-JSON (schema `GET /trap/schema`) naar
/// [`Uitvoer`]: toetsing, tekening, IFC met clearance en IFC voor de leverancier.
///
/// Foutmapping: geen login -> 401; geen entitlement -> 403; ongeldige invoer
/// of geen oplossing -> 422 met `detail`; ongeldig trapmodel of
/// blocking-task-fout -> 500.
async fn trap_ontwerp(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    Json(body): Json<Value>,
) -> Response {
    if let Err(e) = toegang(&state, &claims) {
        return e.into_response();
    }
    let invoer: Invoer = match serde_json::from_value(body) {
        Ok(i) => i,
        Err(e) => {
            return fout(StatusCode::UNPROCESSABLE_ENTITY, "trap_invoer", format!("ongeldige invoer: {e}"))
        }
    };
    let tijd = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    match tokio::task::spawn_blocking(move || ontwerp_uitvoer(&invoer, &tijd)).await {
        Ok(Ok(u)) => Json::<Uitvoer>(u).into_response(),
        Ok(Err(e @ Fout::Model(_))) => fout(StatusCode::INTERNAL_SERVER_ERROR, "trap_model", e.to_string()),
        Ok(Err(e)) => fout(StatusCode::UNPROCESSABLE_ENTITY, "trap_ontwerp", e.to_string()),
        Err(e) => fout(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", e.to_string()),
    }
}

/// GET /trap/schema: JSON-schema van de invoer.
async fn trap_schema(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
) -> Result<Response, ApiError> {
    toegang(&state, &claims)?;
    Ok(([(header::CONTENT_TYPE, "application/json")], trap_core::invoer::schema()).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlements::{DEFAULT_CONSTRUCTIE_GROUP, DEFAULT_TRAP_GROUP};
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    async fn app() -> Router {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        let state = AppState::new(db, None, None, None);
        trap_routes().with_state(state)
    }

    async fn verzoek(methode: &str, pad: &str, groepen: Option<&str>, body: Value) -> (StatusCode, Value) {
        let mut b = Request::builder()
            .method(methode)
            .uri(pad)
            .header("Content-Type", "application/json");
        if let Some(g) = groepen {
            b = b.header("X-Authentik-Username", "tester").header("X-Authentik-Groups", g);
        }
        let req = b.body(Body::from(body.to_string())).expect("request");
        let resp = app().await.oneshot(req).await.expect("response");
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
    }

    async fn post(body: Value) -> (StatusCode, Value) {
        verzoek("POST", "/trap/ontwerp", Some(DEFAULT_TRAP_GROUP), body).await
    }

    fn recht() -> Value {
        serde_json::json!({
            "naam": "recht", "niveaus": {"onder": 0, "boven": 2800},
            "plattegrond": {"soort": "armen", "start": [0, 0], "richting": [1, 0],
                            "binnenzijde": "links", "armlengtes": [3300], "breedte": 850}
        })
    }

    #[tokio::test]
    async fn rechte_trap_geeft_uitvoer() {
        let (status, v) = post(recht()).await;
        assert_eq!(status, StatusCode::OK, "{v}");
        assert!(v["ifc"].as_str().unwrap().contains("IFCBUILDINGELEMENTPROXY("));
        assert!(!v["ifc_leverancier"].as_str().unwrap().contains("IFCBUILDINGELEMENTPROXY("));
        assert!(v["rapport"]["regels"].as_array().unwrap().len() > 5);
        assert!(!v["tekening"]["treden"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn zonder_login_is_401() {
        let (status, _) = verzoek("POST", "/trap/ontwerp", None, recht()).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = verzoek("GET", "/trap/schema", None, Value::Null).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn zonder_entitlement_is_403() {
        // ingelogd, maar alleen het constructierecht
        let (status, v) = verzoek("POST", "/trap/ontwerp", Some(DEFAULT_CONSTRUCTIE_GROUP), recht()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{v}");
        assert_eq!(v["error"], GEEN_TOEGANG);
        let (status, _) = verzoek("GET", "/trap/schema", Some("andere"), Value::Null).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn schema_met_entitlement() {
        let (status, v) = verzoek("GET", "/trap/schema", Some(DEFAULT_TRAP_GROUP), Value::Null).await;
        assert_eq!(status, StatusCode::OK);
        assert!(v["properties"]["looplijn"].is_object(), "{v}");
    }

    #[tokio::test]
    async fn onbekend_veld_is_422() {
        let (status, v) = post(serde_json::json!({"naam": "x", "onzin": 1})).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(v["error"], "trap_invoer");
    }

    #[tokio::test]
    async fn oude_looplijnsleutel_is_422() {
        // v1 mat vanaf de rand van de loopbreedte; die sleutel bestaat niet meer
        let mut b = recht();
        b["looplijn"] = serde_json::json!({"afstand_buitenzijde": 300});
        let (status, v) = post(b).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{v}");
    }

    #[tokio::test]
    async fn binnenhoek_zonder_paal_is_422() {
        let (status, v) = post(serde_json::json!({
            "naam": "kwart", "niveaus": {"onder": 0, "boven": 2800},
            "plattegrond": {"soort": "armen", "start": [0, 0], "richting": [0, 1],
                            "binnenzijde": "links", "armlengtes": [2400, 1600], "breedte": 850}
        }))
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{v}");
        assert!(v["detail"].as_str().unwrap().contains("trappaal"));
    }
}
