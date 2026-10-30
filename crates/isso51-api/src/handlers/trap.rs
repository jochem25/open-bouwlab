//! Trapontwerper (O-2026-10-30-14): invoer naar toetsing, plattegrond en IFC.
//!
//! Publieke reken-route analoog aan `/beng/calculate` (geen auth, wel de
//! body-limit en per-IP rate-limit van de reken-routes). De rekenkern staat in
//! `trap-core`; dit bestand doet alleen HTTP.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;
use trap_core::{ontwerp_uitvoer, Fout, Invoer, Uitvoer};

fn fout(status: StatusCode, code: &str, detail: String) -> Response {
    (status, Json(serde_json::json!({ "error": code, "detail": detail }))).into_response()
}

/// POST /trap/ontwerp: invoer-JSON (schema `GET /trap/schema`) naar
/// [`Uitvoer`]: toetsing, tekening, IFC met clearance en IFC voor de leverancier.
///
/// Foutmapping: ongeldige invoer of geen oplossing -> 422 met `detail`;
/// ongeldig trapmodel of blocking-task-fout -> 500.
pub async fn trap_ontwerp(Json(body): Json<Value>) -> Response {
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
pub async fn trap_schema() -> Response {
    ([(header::CONTENT_TYPE, "application/json")], trap_core::invoer::schema()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn post(body: Value) -> (StatusCode, Value) {
        let r = trap_ontwerp(Json(body)).await;
        let status = r.status();
        let bytes = axum::body::to_bytes(r.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn rechte_trap_geeft_uitvoer() {
        let (status, v) = post(serde_json::json!({
            "naam": "recht", "niveaus": {"onder": 0, "boven": 2800},
            "plattegrond": {"soort": "armen", "start": [0, 0], "richting": [1, 0],
                            "binnenzijde": "links", "armlengtes": [3300], "breedte": 850}
        }))
        .await;
        assert_eq!(status, StatusCode::OK, "{v}");
        assert!(v["ifc"].as_str().unwrap().contains("IFCBUILDINGELEMENTPROXY("));
        assert!(!v["ifc_leverancier"].as_str().unwrap().contains("IFCBUILDINGELEMENTPROXY("));
        assert!(v["rapport"]["regels"].as_array().unwrap().len() > 5);
        assert!(!v["tekening"]["treden"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn onbekend_veld_is_422() {
        let (status, v) = post(serde_json::json!({"naam": "x", "onzin": 1})).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(v["error"], "trap_invoer");
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
