//! beng-import v1 handler.
//!
//! Ontvangt een beng-import-document (contract van 12-10-2026, zie
//! `openaec_project_shared::beng_import`) en geeft de BENG-geometrie terug
//! voor `project.beng_geometry`. De handler rekent niets; de client zet de
//! geometrie in het project en roept daarna `/beng/calculate` aan.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use openaec_project_shared::beng_import::import_beng;

/// `POST /import/beng` — importeer een beng-import-v1-document.
///
/// - 200: `BengImportResult` (`status` compleet of onvolledig);
/// - 400: geen geldige JSON;
/// - 422: JSON klopt niet met het contract (envelope, schema, validatie), met
///   de letterlijke melding inclusief JSON-pad.
pub async fn beng_import_handler(body: String) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let doc: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| (StatusCode::BAD_REQUEST, "json_parse_error", e.to_string()))?;
        import_beng(&doc).map_err(|e| {
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                "beng_import_error",
                e.to_string(),
            )
        })
    })
    .await;

    match result {
        Ok(Ok(r)) => (StatusCode::OK, Json(r)).into_response(),
        Ok(Err((status, error, detail))) => (
            status,
            Json(serde_json::json!({ "error": error, "detail": detail })),
        )
            .into_response(),
        Err(join_err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "internal_error", "detail": join_err.to_string() })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../../tests/fixtures/beng-import-v1-synthetisch.json");

    async fn call(body: &str) -> (StatusCode, serde_json::Value) {
        let resp = beng_import_handler(body.to_string()).await.into_response();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn synthetische_fixture_geeft_200_compleet() {
        let (status, v) = call(FIXTURE).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(v["status"], "compleet");
        assert_eq!(v["beng_geometry"]["zones"][0]["id"], "rz-1");
    }

    #[tokio::test]
    async fn ongeldige_json_geeft_400() {
        let (status, v) = call("{niet json").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(v["error"], "json_parse_error");
    }

    #[tokio::test]
    async fn contractfout_geeft_422_met_pad() {
        let doc = FIXTURE.replace("\"vlak_type\": \"vloer\"", "\"vlak_type\": \"muur\"");
        let (status, v) = call(&doc).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(v["detail"]
            .as_str()
            .unwrap()
            .contains("beng_geometry.zones[0].gevels[0].vlak_type"));
    }
}
