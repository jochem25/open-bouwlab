//! Report generation proxy handler.
//!
//! Forwards report JSON to the OpenAEC Reports API via Authentik service-token
//! (``svc-warmteverlies``).
//!
//! Organisatie en huisstijl (zie `crate::organisatie`):
//! - Zonder organisatie geen rapport: 403 `geen_organisatie` (of
//!   `meerdere_organisaties` / `onbekende_organisatie`). Geen stille fallback.
//! - `brand` in de body is de gekozen huisstijl. Leeg = de standaardhuisstijl
//!   van de organisatie; buiten de toegestane lijst = 403
//!   `huisstijl_niet_toegestaan`. De proxy zet de gevalideerde `brand` altijd
//!   in de body en stuurt `X-Original-Tenant: <organisatie-id>`.
//!
//! Auth vereisten:
//! - Caller moet via forward_auth (AuthClaims) authenticated zijn
//! - Upstream-call gebruikt service-token Bearer als geconfigureerd
//! - Fallback: legacy X-API-Key als service-token niet beschikbaar is
//!
//! Standaard-coverfoto: bevat het rapport geen `cover.image`, dan vult de
//! proxy die aan uit `REPORT_DEFAULT_COVER_DIR/<organisatie-id>.{jpg,png}`.
//! Zo werkt een sjabloon met een verplichte foto (KBA) ook zonder upload,
//! zonder dat het bedrijfsbeeld in deze publieke repo staat. Geen bestand voor
//! de organisatie: rapport gaat ongewijzigd door en een afkeuring van de
//! Reports API komt met de oorspronkelijke tekst bij de gebruiker.

use std::path::Path;
use std::time::Duration;

use base64::Engine;
use serde_json::{Map, Value};

use axum::extract::State;
use axum::http::header;
use axum::response::{IntoResponse, Response};

use crate::auth::{AuthClaims, OidcClaims};
use crate::error::ApiError;
use crate::organisatie::Organisatie;
use crate::state::AppState;

/// POST /report/generate — proxy report generation to OpenAEC Reports API.
///
/// Auth chain:
/// 1. `AuthClaims` extractor (forward_auth) valideert de caller; de
///    organisatie volgt uit zijn `org-*`-groep.
/// 2. Upstream call gebruikt `REPORTS_API_SERVICE_TOKEN` (Authentik ak-*) als
///    primary auth methode, met `X-Original-Tenant: <organisatie-id>` en de
///    gevalideerde `brand` in de body.
/// 3. Fallback voor transitie: als service-token niet geconfigureerd is maar
///    `REPORTS_API_KEY` wel, stuur die als `X-API-Key` (legacy Caddy bypass).
pub async fn generate_report(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    body: String,
) -> Result<Response, ApiError> {
    proxy_report(&state, &claims, body).await
}

/// Organisatie van de gebruiker; zonder (geldige) organisatie een 403.
fn organisatie_voor_rapport<'a>(
    state: &'a AppState,
    claims: &OidcClaims,
) -> Result<&'a Organisatie, ApiError> {
    match state.organisaties.van_claims(claims) {
        Ok(Some(org)) => Ok(org),
        Ok(None) => Err(ApiError::Organisatie {
            code: "geen_organisatie",
            detail: "Je account is niet aan een organisatie gekoppeld; vraag de beheerder".to_string(),
        }),
        Err(fout) => Err(ApiError::Organisatie {
            code: fout.code(),
            detail: fout.detail(),
        }),
    }
}

/// Bepaal de huisstijl uit `brand` in het rapport en zet hem gevalideerd terug.
///
/// Ontbreekt `brand` (of is hij leeg/null), dan de standaard van de organisatie.
/// Een huisstijl buiten de toegestane lijst is een 403, geen fallback.
fn zet_huisstijl(rapport: &mut Map<String, Value>, org: &Organisatie) -> Result<String, ApiError> {
    let gekozen = match rapport.get("brand") {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(Value::String(s)) => Some(s.trim().to_string()),
        Some(_) => {
            return Err(ApiError::BadRequest("'brand' moet een tekst zijn".to_string()));
        }
    };
    let huisstijl = match gekozen {
        None => org.standaard_huisstijl.clone(),
        Some(h) if org.mag_huisstijl(&h) => h,
        Some(h) => {
            return Err(ApiError::Organisatie {
                code: "huisstijl_niet_toegestaan",
                detail: format!(
                    "Huisstijl '{h}' is niet toegestaan voor {}; kies uit: {}",
                    org.naam,
                    org.huisstijlen.join(", ")
                ),
            });
        }
    };
    rapport.insert("brand".to_string(), Value::String(huisstijl.clone()));
    // De tenant bepaalt de proxy (X-Original-Tenant), nooit de client.
    rapport.remove("tenant");
    Ok(huisstijl)
}

/// Vul de auteur in het colofon met de ingelogde gebruiker en zijn organisatie.
///
/// De Reports API vult lege `adviseur_*`-velden met het profiel van de
/// geauthenticeerde gebruiker; via deze proxy is dat het service-account, dat
/// dan als "Opgesteld door" in het rapport kwam. Daarom vult de proxy ze zelf:
/// naam (weergavenaam, anders gebruikersnaam), bedrijf (organisatienaam) en
/// e-mail. Een veld dat de frontend al invulde blijft staan.
fn vul_auteur(rapport: &mut Map<String, Value>, claims: &OidcClaims, org: &Organisatie) {
    let colofon = rapport
        .entry("colofon")
        .or_insert_with(|| Value::Object(Map::new()));
    if !colofon.is_object() {
        *colofon = Value::Object(Map::new());
    }
    let Some(colofon) = colofon.as_object_mut() else {
        return;
    };
    let naam = claims
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(claims.sub.as_str());
    let email = claims.email.as_deref().map(str::trim).filter(|e| !e.is_empty());
    for (veld, waarde) in [
        ("adviseur_naam", Some(naam)),
        ("adviseur_bedrijf", Some(org.naam.as_str())),
        ("adviseur_email", email),
    ] {
        let leeg = match colofon.get(veld) {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.trim().is_empty(),
            Some(_) => false,
        };
        if let (true, Some(w)) = (leeg, waarde.filter(|w| !w.is_empty())) {
            colofon.insert(veld.to_string(), Value::String(w.to_string()));
        }
    }
}

/// Stuur rapport-JSON door naar de Reports API en geef de PDF terug.
///
/// Gedeeld door `/report/generate` en de constructiemodule; het gedrag
/// (organisatie, huisstijl, service-token, foutmapping) is identiek.
pub(crate) async fn proxy_report(
    state: &AppState,
    claims: &OidcClaims,
    body: String,
) -> Result<Response, ApiError> {
    let org = organisatie_voor_rapport(state, claims)?;

    let base_url = state.reports_api_url.as_deref().ok_or_else(|| {
        ApiError::ServiceUnavailable(
            "Rapportgeneratie is niet geconfigureerd (REPORTS_API_URL ontbreekt)".to_string(),
        )
    })?;

    let mut rapport = match serde_json::from_str::<Value>(&body) {
        Ok(Value::Object(map)) => map,
        _ => {
            return Err(ApiError::BadRequest(
                "Rapport moet een JSON-object zijn".to_string(),
            ));
        }
    };
    drop(body);
    let huisstijl = zet_huisstijl(&mut rapport, org)?;
    vul_auteur(&mut rapport, claims, org);

    let url = format!("{}/api/generate/v2", base_url.trim_end_matches('/'));

    let mut req = state
        .http_client
        .post(&url)
        .header(header::CONTENT_TYPE.as_str(), "application/json")
        .timeout(Duration::from_secs(30));

    // Primair: Authentik service-token (Bearer ak-*) + X-Original-Tenant
    if let Some(token) = state.reports_api_service_token.as_deref() {
        req = req
            .header(header::AUTHORIZATION.as_str(), format!("Bearer {token}"))
            .header("X-Original-Tenant", org.id.as_str());
    } else if let Some(api_key) = state.reports_api_key.as_deref() {
        // Legacy fallback: X-API-Key (Caddy bypass) — wordt verwijderd
        // zodra service-token overal werkt.
        req = req.header("X-API-Key", api_key);
    }
    tracing::info!(user = %claims.sub, organisatie = %org.id, huisstijl = %huisstijl, "rapport naar Reports API");

    let org_id = org.id.clone();
    let dir = state.report_default_cover_dir.clone();
    let body = tokio::task::spawn_blocking(move || {
        vul_standaard_cover(&mut rapport, &org_id, dir.as_deref());
        Value::Object(rapport).to_string()
    })
    .await
    .map_err(|e| ApiError::Internal(format!("standaard-coverfoto: {e}")))?;
    let upstream = req.body(body).send().await.map_err(|e| {
        tracing::error!("Reports API request failed: {e}");
        ApiError::ReportService(format!("Rapport service niet bereikbaar: {e}"))
    })?;

    if !upstream.status().is_success() {
        let status = upstream.status();
        let detail = upstream.text().await.unwrap_or_default();
        tracing::error!("Reports API returned {status}: {detail}");
        if is_inhoudelijke_afkeuring(status) {
            return Err(ApiError::ReportRejected {
                status,
                detail: format!(
                    "Rapport afgekeurd door de rapportengine: {}",
                    upstream_detail(&detail)
                ),
            });
        }
        return Err(ApiError::ReportService(format!(
            "Rapport generatie mislukt ({status}): {detail}"
        )));
    }

    let pdf_bytes = upstream.bytes().await.map_err(|e| {
        tracing::error!("Failed to read report PDF: {e}");
        ApiError::ReportService("Fout bij ophalen van rapport PDF".to_string())
    })?;

    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"rapport.pdf\"",
            ),
        ],
        pdf_bytes,
    )
        .into_response())
}

/// Maximale grootte van een standaard-coverfoto (bytes, ongecodeerd).
const MAX_COVER_BYTES: u64 = 5 * 1024 * 1024;

/// Vul `cover.image` aan met de standaardfoto van de organisatie als die ontbreekt.
///
/// Laat het rapport ongewijzigd als er al een afbeelding is, als er geen map
/// is of als het bestand ontbreekt.
pub(crate) fn vul_standaard_cover(rapport: &mut Map<String, Value>, org_id: &str, dir: Option<&Path>) {
    let Some(dir) = dir else {
        return;
    };
    if !crate::organisatie::is_slug(org_id) {
        tracing::warn!(org_id, "organisatie-id ongeschikt als bestandsnaam: geen standaard-coverfoto");
        return;
    }
    let heeft_foto = rapport
        .get("cover")
        .and_then(|c| c.get("image"))
        .is_some_and(|i| !i.is_null());
    if heeft_foto {
        return;
    }
    let Some(afbeelding) = lees_standaard_cover(dir, org_id) else {
        tracing::info!(org_id, "geen standaard-coverfoto voor organisatie");
        return;
    };
    let cover = rapport
        .entry("cover")
        .or_insert_with(|| Value::Object(Default::default()));
    if !cover.is_object() {
        // `cover: null` of een andere vorm: vervangen, anders zou de foto stil wegvallen.
        *cover = Value::Object(Default::default());
    }
    if let Some(c) = cover.as_object_mut() {
        c.insert("image".to_string(), afbeelding);
    }
}

/// Lees `<dir>/<org_id>.jpg` of `.png` als base64-afbeelding voor de Reports API.
fn lees_standaard_cover(dir: &Path, org_id: &str) -> Option<Value> {
    for (ext, media_type) in [("jpg", "image/jpeg"), ("jpeg", "image/jpeg"), ("png", "image/png")] {
        let pad = dir.join(format!("{org_id}.{ext}"));
        let Ok(meta) = std::fs::metadata(&pad) else {
            continue;
        };
        if meta.len() > MAX_COVER_BYTES {
            tracing::error!(pad = %pad.display(), "standaard-coverfoto te groot; overgeslagen");
            return None;
        }
        match std::fs::read(&pad) {
            Ok(bytes) => {
                return Some(serde_json::json!({
                    "data": base64::engine::general_purpose::STANDARD.encode(bytes),
                    "media_type": media_type,
                    "filename": format!("standaard-{org_id}.{ext}"),
                }));
            }
            Err(e) => {
                tracing::error!(pad = %pad.display(), "standaard-coverfoto onleesbaar: {e}");
                return None;
            }
        }
    }
    None
}

/// Haal de leesbare tekst uit een foutbody van de Reports API.
///
/// FastAPI geeft `{"detail": "..."}` of, bij schemafouten, `{"detail": [{"msg": ...}]}`.
/// Anders de ruwe tekst, ingekort.
pub(crate) fn upstream_detail(body: &str) -> String {
    const GEEN_TOELICHTING: &str = "geen toelichting ontvangen";
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        // Geen JSON (bijv. een HTML-foutpagina van een proxy): niet tonen.
        return GEEN_TOELICHTING.to_string();
    };
    let tekst = match json.get("detail") {
        Some(Value::String(s)) => s.clone(),
        // Alleen `msg`: een schemafout bevat ook `input`, mogelijk de hele afbeelding.
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|i| i.get("msg").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("; "),
        _ => String::new(),
    };
    let tekst = tekst.trim();
    if tekst.is_empty() {
        return GEEN_TOELICHTING.to_string();
    }
    kap_af(tekst, MAX_DETAIL_TEKENS)
}

/// Maximale lengte van een doorgegeven fouttekst (tekens).
const MAX_DETAIL_TEKENS: usize = 500;

/// Kap `tekst` af op `max` tekens (op een tekengrens).
fn kap_af(tekst: &str, max: usize) -> String {
    match tekst.char_indices().nth(max) {
        Some((i, _)) => format!("{}...", &tekst[..i]),
        None => tekst.to_string(),
    }
}

/// 400, 413 en 422 zeggen iets over het rapport zelf en gaan door naar de
/// gebruiker. Andere 4xx (401/403 service-token, 404 verkeerde URL, 429) zijn
/// configuratie- of serverfouten: die blijven 502, anders leest de frontend
/// een 401/403 als een verlopen sessie van de gebruiker.
fn is_inhoudelijke_afkeuring(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 400 | 413 | 422)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use axum::http::{HeaderMap, StatusCode};

    /// Verse map met een standaardfoto `kba.jpg` (inhoud is willekeurig).
    fn cover_map(naam: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("isso51-cover-{naam}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("map");
        std::fs::write(dir.join("kba.jpg"), b"jpeg-bytes").expect("foto");
        dir
    }

    fn object(json: &str) -> Map<String, Value> {
        match serde_json::from_str::<Value>(json).expect("json") {
            Value::Object(m) => m,
            _ => panic!("geen object"),
        }
    }

    fn cover_image(body: &str) -> Value {
        serde_json::from_str::<Value>(body).expect("json")["cover"]["image"].clone()
    }

    #[test]
    fn zonder_upload_vult_standaardfoto_van_organisatie_aan() {
        let dir = cover_map("zonder");
        let mut r = object(r#"{"template":"standaard_rapport","cover":{"subtitle":"x"}}"#);
        vul_standaard_cover(&mut r, "kba", Some(&dir));
        let image = &r["cover"]["image"];
        assert_eq!(image["media_type"], "image/jpeg");
        assert_eq!(image["data"], base64::engine::general_purpose::STANDARD.encode(b"jpeg-bytes"));
        assert_eq!(r["cover"]["subtitle"], "x");

        // Ook zonder cover-object, met `cover: null` en met `image: null`.
        for body in [
            r#"{"template":"t"}"#,
            r#"{"template":"t","cover":null}"#,
            r#"{"template":"t","cover":{"image":null}}"#,
        ] {
            let mut r = object(body);
            vul_standaard_cover(&mut r, "kba", Some(&dir));
            assert_eq!(r["cover"]["image"]["media_type"], "image/jpeg", "{body}");
        }
    }

    #[test]
    fn met_upload_blijft_rapport_ongewijzigd() {
        let dir = cover_map("met");
        let origineel = object(r#"{"cover":{"image":{"data":"QUJD","media_type":"image/png"}}}"#);
        let mut r = origineel.clone();
        vul_standaard_cover(&mut r, "kba", Some(&dir));
        assert_eq!(r, origineel);
    }

    #[test]
    fn geen_standaard_voor_organisatie_of_geen_map_laat_rapport_ongewijzigd() {
        let dir = cover_map("geen");
        let origineel = object(r#"{"cover":{}}"#);
        for (org, map) in [("ander", Some(dir.as_path())), ("kba", None), ("../kba", Some(dir.as_path()))] {
            let mut r = origineel.clone();
            vul_standaard_cover(&mut r, org, map);
            assert_eq!(r, origineel, "{org}");
        }
    }

    #[test]
    fn upstream_detail_leest_fastapi_fouten() {
        assert_eq!(upstream_detail(r#"{"detail":"foto ontbreekt"}"#), "foto ontbreekt");
        assert_eq!(
            upstream_detail(r#"{"detail":[{"msg":"a"},{"msg":"b"}]}"#),
            "a; b"
        );
        // Zonder `msg` geen ruwe items (die kunnen de afbeelding als `input` bevatten).
        assert_eq!(
            upstream_detail(r#"{"detail":[{"msg":"a","input":"QUJD"},{"loc":["x"]}]}"#),
            "a"
        );
        // Geen JSON (HTML-foutpagina van een proxy): generieke tekst, niets doorlekken.
        assert_eq!(upstream_detail("<html>intern</html>"), "geen toelichting ontvangen");
        assert_eq!(upstream_detail(""), "geen toelichting ontvangen");
        let lang = upstream_detail(&format!(r#"{{"detail":"{}"}}"#, "x".repeat(2000)));
        assert!(lang.ends_with("...") && lang.chars().count() == MAX_DETAIL_TEKENS + 3);
    }

    /// Wat de nep-Reports API ontving: body en headers.
    #[derive(Default)]
    struct Ontvangen {
        body: String,
        headers: HeaderMap,
        aantal: usize,
    }

    /// Nep-Reports API: bewaart body en headers, antwoordt met `status` + `antwoord`.
    async fn nep_reports_api(status: StatusCode, antwoord: &'static str) -> (String, Arc<Mutex<Ontvangen>>) {
        let ontvangen = Arc::new(Mutex::new(Ontvangen::default()));
        let opslag = ontvangen.clone();
        let app = axum::Router::new().route(
            "/api/generate/v2",
            axum::routing::post(move |headers: HeaderMap, body: String| {
                let opslag = opslag.clone();
                async move {
                    let mut o = opslag.lock().expect("lock");
                    o.body = body;
                    o.headers = headers;
                    o.aantal += 1;
                    (status, [(header::CONTENT_TYPE, "application/json")], antwoord)
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let adres = listener.local_addr().expect("adres");
        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });
        (format!("http://{adres}"), ontvangen)
    }

    async fn state(url: String, dir: Option<PathBuf>) -> AppState {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        let mut state = AppState::new(db, Some(url), None, Some("ak-test".to_string()));
        state.report_default_cover_dir = dir;
        state.organisaties = Arc::new(crate::organisatie::test_organisaties());
        state
    }

    fn met_groepen(groepen: &[&str]) -> OidcClaims {
        OidcClaims {
            sub: "tester".to_string(),
            groups: groepen.iter().map(|g| (*g).to_string()).collect(),
            ..Default::default()
        }
    }

    fn kba() -> OidcClaims {
        met_groepen(&["org-kba"])
    }

    async fn fout_json(fout: ApiError) -> (StatusCode, Value) {
        let resp = fout.into_response();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.expect("body");
        (status, serde_json::from_slice(&bytes).expect("json"))
    }

    #[tokio::test]
    async fn rapport_met_organisatie_kba_krijgt_standaardhuisstijl_en_tenantheader() {
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        let resp = proxy_report(&state(url, None).await, &kba(), r#"{"project":"p"}"#.to_string())
            .await
            .expect("pdf");
        assert_eq!(resp.status(), StatusCode::OK);
        let o = ontvangen.lock().expect("lock");
        let body: Value = serde_json::from_str(&o.body).expect("json");
        assert_eq!(body["brand"], "kba");
        assert_eq!(body["project"], "p");
        assert_eq!(o.headers["x-original-tenant"], "kba");
        assert_eq!(o.headers["authorization"], "Bearer ak-test");
    }

    #[tokio::test]
    async fn gekozen_toegestane_huisstijl_gaat_door() {
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        proxy_report(
            &state(url, None).await,
            &kba(),
            r#"{"project":"p","brand":"openaec_foundation"}"#.to_string(),
        )
        .await
        .expect("pdf");
        let o = ontvangen.lock().expect("lock");
        let body: Value = serde_json::from_str(&o.body).expect("json");
        assert_eq!(body["brand"], "openaec_foundation");
        assert_eq!(o.headers["x-original-tenant"], "kba");
    }

    #[tokio::test]
    async fn tenant_uit_de_body_gaat_niet_mee() {
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        proxy_report(&state(url, None).await, &kba(), r#"{"tenant":"andere"}"#.to_string())
            .await
            .expect("pdf");
        let o = ontvangen.lock().expect("lock");
        let body: Value = serde_json::from_str(&o.body).expect("json");
        assert!(body.get("tenant").is_none());
        assert_eq!(o.headers["x-original-tenant"], "kba");
    }

    #[tokio::test]
    async fn lege_of_null_brand_is_standaard() {
        for body in [r#"{"brand":""}"#, r#"{"brand":null}"#, r#"{"brand":"  "}"#] {
            let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
            proxy_report(&state(url, None).await, &kba(), body.to_string())
                .await
                .expect("pdf");
            let o = ontvangen.lock().expect("lock");
            assert_eq!(serde_json::from_str::<Value>(&o.body).expect("json")["brand"], "kba", "{body}");
        }
    }

    #[tokio::test]
    async fn niet_toegestane_huisstijl_is_403_zonder_upstream_call() {
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        let fout = proxy_report(&state(url, None).await, &kba(), r#"{"brand":"3bm"}"#.to_string())
            .await
            .expect_err("403");
        let (status, json) = fout_json(fout).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"], "huisstijl_niet_toegestaan");
        let detail = json["detail"].as_str().expect("detail");
        assert!(detail.contains("'3bm'") && detail.contains("kba, openaec_foundation"), "{detail}");
        assert_eq!(ontvangen.lock().expect("lock").aantal, 0);
    }

    #[tokio::test]
    async fn zonder_geldige_organisatie_geen_rapport() {
        let gevallen: [(&[&str], &str); 3] = [
            (&["openbouwlab-constructie"], "geen_organisatie"),
            (&["org-kba", "org-andere"], "meerdere_organisaties"),
            (&["org-onbekend"], "onbekende_organisatie"),
        ];
        for (groepen, code) in gevallen {
            let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
            let fout = proxy_report(&state(url, None).await, &met_groepen(groepen), "{}".to_string())
                .await
                .expect_err("403");
            let (status, json) = fout_json(fout).await;
            assert_eq!(status, StatusCode::FORBIDDEN, "{code}");
            assert_eq!(json["error"], code);
            assert!(!json["detail"].as_str().unwrap_or("").is_empty());
            assert_eq!(ontvangen.lock().expect("lock").aantal, 0, "{code}");
        }
    }

    #[tokio::test]
    async fn geen_json_object_of_brand_geen_tekst_is_400() {
        for body in ["geen json", "[1,2]", r#"{"brand":3}"#] {
            let (url, _) = nep_reports_api(StatusCode::OK, "%PDF").await;
            let fout = proxy_report(&state(url, None).await, &kba(), body.to_string())
                .await
                .expect_err("400");
            assert_eq!(fout.into_response().status(), StatusCode::BAD_REQUEST, "{body}");
        }
    }

    #[tokio::test]
    async fn afkeuring_4xx_komt_met_detailtekst_bij_gebruiker() {
        let (url, _) = nep_reports_api(
            StatusCode::UNPROCESSABLE_ENTITY,
            r#"{"detail":"static_elements: verplichte afbeelding ontbreekt"}"#,
        )
        .await;
        let fout = proxy_report(&state(url, None).await, &kba(), r#"{"cover":{}}"#.to_string())
            .await
            .expect_err("422");
        let (status, json) = fout_json(fout).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(json["error"], "report_rejected");
        assert_eq!(
            json["detail"],
            "Rapport afgekeurd door de rapportengine: static_elements: verplichte afbeelding ontbreekt"
        );
    }

    #[tokio::test]
    async fn auth_fout_van_reports_api_blijft_serverfout() {
        // Een verlopen service-token mag bij de gebruiker geen 401 worden.
        for status in [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN, StatusCode::NOT_FOUND] {
            let (url, _) = nep_reports_api(status, r#"{"detail":"x"}"#).await;
            let fout = proxy_report(&state(url, None).await, &kba(), "{}".to_string())
                .await
                .expect_err("fout");
            assert_eq!(fout.into_response().status(), StatusCode::BAD_GATEWAY, "{status}");
        }
    }

    #[tokio::test]
    async fn fout_5xx_blijft_report_service_error() {
        let (url, _) = nep_reports_api(StatusCode::INTERNAL_SERVER_ERROR, "boem").await;
        let fout = proxy_report(&state(url, None).await, &kba(), "{}".to_string())
            .await
            .expect_err("500");
        assert_eq!(fout.into_response().status(), StatusCode::BAD_GATEWAY);
    }

    fn colofon_van(body: &str) -> Value {
        serde_json::from_str::<Value>(body).expect("json")["colofon"].clone()
    }

    #[tokio::test]
    async fn colofon_krijgt_gebruiker_en_organisatie_als_auteur() {
        let gebruiker = OidcClaims {
            name: Some("Ingrid Tester".to_string()),
            email: Some("ingrid@example.org".to_string()),
            ..kba()
        };
        // Ontbrekend colofon, leeg colofon, lege strings (frontend zonder engineer) en null.
        for body in [
            r#"{"project":"p"}"#,
            r#"{"project":"p","colofon":null}"#,
            r#"{"project":"p","colofon":"geen object"}"#,
            r#"{"project":"p","colofon":{"enabled":false}}"#,
            r#"{"project":"p","colofon":{"adviseur_naam":"","adviseur_bedrijf":" ","adviseur_email":null}}"#,
        ] {
            let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
            proxy_report(&state(url, None).await, &gebruiker, body.to_string())
                .await
                .expect("pdf");
            let colofon = colofon_van(&ontvangen.lock().expect("lock").body);
            assert_eq!(colofon["adviseur_naam"], "Ingrid Tester", "{body}");
            assert_eq!(colofon["adviseur_bedrijf"], "Testbureau", "{body}");
            assert_eq!(colofon["adviseur_email"], "ingrid@example.org", "{body}");
            // Een uitgeschakeld colofon blijft uitgeschakeld.
            let verwacht_enabled = if body.contains("enabled") { Value::Bool(false) } else { Value::Null };
            assert_eq!(colofon["enabled"], verwacht_enabled, "{body}");
        }
    }

    #[tokio::test]
    async fn colofon_zonder_weergavenaam_krijgt_gebruikersnaam() {
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        let gebruiker = OidcClaims { name: Some("  ".to_string()), ..kba() };
        proxy_report(&state(url, None).await, &gebruiker, r#"{"project":"p"}"#.to_string())
            .await
            .expect("pdf");
        let colofon = colofon_van(&ontvangen.lock().expect("lock").body);
        assert_eq!(colofon["adviseur_naam"], "tester");
        assert!(colofon.get("adviseur_email").is_none());
    }

    #[tokio::test]
    async fn colofon_van_de_frontend_blijft_staan() {
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        let gebruiker = OidcClaims {
            name: Some("Ingrid Tester".to_string()),
            email: Some("ingrid@example.org".to_string()),
            ..kba()
        };
        let body = r#"{"project":"p","colofon":{"adviseur_naam":"Ir. Ander","adviseur_bedrijf":"Eigen BV","adviseur_email":"ander@example.org","fase":"DO"}}"#;
        proxy_report(&state(url, None).await, &gebruiker, body.to_string())
            .await
            .expect("pdf");
        let colofon = colofon_van(&ontvangen.lock().expect("lock").body);
        assert_eq!(colofon["adviseur_naam"], "Ir. Ander");
        assert_eq!(colofon["adviseur_bedrijf"], "Eigen BV");
        assert_eq!(colofon["adviseur_email"], "ander@example.org");
        assert_eq!(colofon["fase"], "DO");
    }

    #[tokio::test]
    async fn proxy_stuurt_standaardfoto_mee_zonder_upload() {
        let dir = cover_map("proxy");
        let (url, ontvangen) = nep_reports_api(StatusCode::OK, "%PDF").await;
        proxy_report(&state(url, Some(dir)).await, &kba(), r#"{"cover":{}}"#.to_string())
            .await
            .expect("pdf");
        let body = ontvangen.lock().expect("lock").body.clone();
        assert_eq!(cover_image(&body)["media_type"], "image/jpeg");
    }
}
