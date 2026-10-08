//! Constructiemodule: voordimensionering van liggers (staal, beton, hout).
//!
//! Alle routes vereisen een ingelogde gebruiker (`AuthClaims`, 401 zonder login)
//! en het entitlement `constructie` (403 zonder), zie [`crate::entitlements`].
//! De rekenkern staat in `constructie-core`; dit bestand doet alleen HTTP.

use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use constructie_core::rapport::{rapport_json, RapportBerekening, RapportInvoer};
use constructie_core::{BetonInvoer, HoutInvoer, Materiaal, StaalInvoer};
use serde::Deserialize;
use serde_json::Value;

use crate::auth::{AuthClaims, OidcClaims};
use crate::entitlements::constructie_toegestaan;
use crate::error::ApiError;
use crate::state::AppState;

/// Foutmelding bij ontbrekend entitlement (letterlijk in de response).
const GEEN_TOEGANG: &str = "module constructie niet geactiveerd voor dit account";

/// Maximale grootte van een geuploade coverfoto (bytes, ongecodeerd); gelijk
/// aan de voorbladafbeelding van de warmteverliesrapportage.
const MAX_COVERFOTO_BYTES: usize = 2 * 1024 * 1024;

/// Maximale lengte van de meegestuurde bestandsnaam van de coverfoto.
const MAX_BESTANDSNAAM_TEKENS: usize = 255;

/// Body-limiet van `/constructie/rapport`: coverfoto (base64) plus invoer.
/// Ruimer dan de reken-routes; de route-laag overschrijft de module-laag.
const RAPPORT_BODY_LIMIT: usize = 4 * 1024 * 1024;

/// Standaard projectnaam in het rapport als de client er geen meegeeft.
const STANDAARD_PROJECTNAAM: &str = "Indicatieve voordimensionering";

/// Optionele projectgegevens voor het rapport.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectMeta {
    naam: Option<String>,
    opsteller: Option<String>,
    /// Optionele coverfoto (png/jpeg, base64). Zonder foto vult de rapport-proxy
    /// de standaardfoto van de tenant aan.
    coverfoto: Option<Coverfoto>,
}

/// Geuploade coverfoto, in het formaat dat de Reports API als `cover.image` leest.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Coverfoto {
    data: String,
    media_type: String,
    #[serde(default)]
    filename: Option<String>,
}

impl Coverfoto {
    /// Controleer type en grootte; geef de `cover.image`-waarde terug.
    fn als_cover_image(&self) -> Result<Value, ApiError> {
        use base64::Engine;
        if !matches!(self.media_type.as_str(), "image/png" | "image/jpeg") {
            return Err(ApiError::BadRequest(
                "coverfoto moet een PNG- of JPEG-afbeelding zijn".to_string(),
            ));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(self.data.as_bytes())
            .map_err(|_| ApiError::BadRequest("coverfoto is geen geldige base64".to_string()))?;
        if bytes.len() > MAX_COVERFOTO_BYTES {
            return Err(ApiError::BadRequest("coverfoto is groter dan 2 MB".to_string()));
        }
        // Inhoud moet bij het opgegeven type passen (bestandssignatuur).
        let signatuur_klopt = match self.media_type.as_str() {
            "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            _ => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        };
        if !signatuur_klopt {
            return Err(ApiError::BadRequest(
                "coverfoto is geen geldige PNG- of JPEG-afbeelding".to_string(),
            ));
        }
        let mut image = serde_json::json!({
            "data": self.data,
            "media_type": self.media_type,
        });
        if let Some(naam) = self.filename.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
            image["filename"] = Value::String(naam.chars().take(MAX_BESTANDSNAAM_TEKENS).collect());
        }
        Ok(image)
    }
}

/// Body van `POST /constructie/rapport`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RapportAanvraag {
    materiaal: Materiaal,
    invoer: Value,
    #[serde(default)]
    project: Option<ProjectMeta>,
}

/// Routes van de constructiemodule (relatief aan het API-prefix).
pub fn constructie_routes() -> Router<AppState> {
    Router::new()
        .route("/constructie/staal", post(staal))
        .route("/constructie/beton", post(beton))
        .route("/constructie/hout", post(hout))
        .route(
            "/constructie/rapport",
            post(rapport).layer(DefaultBodyLimit::max(RAPPORT_BODY_LIMIT)),
        )
        .route("/constructie/schema/{naam}", get(schema))
}

/// Controleer het entitlement; anders 403.
fn toegang(state: &AppState, claims: &OidcClaims) -> Result<(), ApiError> {
    if constructie_toegestaan(claims, &state.entitlements) {
        Ok(())
    } else {
        Err(ApiError::ModuleNietGeactiveerd(GEEN_TOEGANG.to_string()))
    }
}

/// POST /constructie/staal — voordimensionering stalen ligger.
async fn staal(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    body: String,
) -> Result<Json<constructie_core::StaalResultaat>, ApiError> {
    toegang(&state, &claims)?;
    let invoer: StaalInvoer =
        serde_json::from_str(&body).map_err(constructie_core::ConstructieFout::from)?;
    Ok(Json(constructie_core::bereken_staal(&invoer)?))
}

/// POST /constructie/beton — voordimensionering betonbalk.
async fn beton(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    body: String,
) -> Result<Json<constructie_core::BetonResultaat>, ApiError> {
    toegang(&state, &claims)?;
    let invoer: BetonInvoer =
        serde_json::from_str(&body).map_err(constructie_core::ConstructieFout::from)?;
    Ok(Json(constructie_core::bereken_beton(&invoer)?))
}

/// POST /constructie/hout — voordimensionering houten balk of balklaag.
async fn hout(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    body: String,
) -> Result<Json<constructie_core::HoutResultaat>, ApiError> {
    toegang(&state, &claims)?;
    let invoer: HoutInvoer =
        serde_json::from_str(&body).map_err(constructie_core::ConstructieFout::from)?;
    Ok(Json(constructie_core::bereken_hout(&invoer)?))
}

/// Rekent opnieuw en bouwt de rapport-JSON. De disclaimer zit altijd in het
/// rapport; de client kan hem niet weglaten.
fn bouw_rapport(body: &str, claims: &OidcClaims) -> Result<Value, ApiError> {
    use constructie_core::ConstructieFout;

    let aanvraag: RapportAanvraag = serde_json::from_str(body).map_err(ConstructieFout::from)?;
    let meta = aanvraag.project.unwrap_or_default();
    let cover_image = meta.coverfoto.as_ref().map(Coverfoto::als_cover_image).transpose()?;
    let project = meta
        .naam
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or(STANDAARD_PROJECTNAAM);
    let auteur = meta
        .opsteller
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .or(claims.name.as_deref())
        .unwrap_or("");
    let datum = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let rapport = |berekening| {
        rapport_json(&RapportInvoer {
            project,
            projectnummer: "",
            opdrachtgever: "",
            auteur,
            datum: &datum,
            berekening,
        })
    };

    let mut json = match aanvraag.materiaal {
        Materiaal::Staal => {
            let invoer: StaalInvoer =
                serde_json::from_value(aanvraag.invoer).map_err(ConstructieFout::from)?;
            let resultaat = constructie_core::bereken_staal(&invoer)?;
            rapport(RapportBerekening::Staal {
                invoer: &invoer,
                resultaat: &resultaat,
            })
        }
        Materiaal::Beton => {
            let invoer: BetonInvoer =
                serde_json::from_value(aanvraag.invoer).map_err(ConstructieFout::from)?;
            let resultaat = constructie_core::bereken_beton(&invoer)?;
            rapport(RapportBerekening::Beton {
                invoer: &invoer,
                resultaat: &resultaat,
            })
        }
        Materiaal::Hout => {
            let invoer: HoutInvoer =
                serde_json::from_value(aanvraag.invoer).map_err(ConstructieFout::from)?;
            let resultaat = constructie_core::bereken_hout(&invoer)?;
            rapport(RapportBerekening::Hout {
                invoer: &invoer,
                resultaat: &resultaat,
            })
        }
    };
    if let Some(image) = cover_image {
        json["cover"]["image"] = image;
    }
    Ok(json)
}

/// POST /constructie/rapport — server-side herberekening en PDF via de rapport-proxy.
async fn rapport(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    body: String,
) -> Result<Response, ApiError> {
    toegang(&state, &claims)?;
    let json = bouw_rapport(&body, &claims)?;
    super::report::proxy_report(&state, &claims, json.to_string()).await
}

/// GET /constructie/schema/{naam} — JSON-schema (`staal-invoer`, `beton-invoer`, `hout-invoer`, `resultaat`).
async fn schema(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
    Path(naam): Path<String>,
) -> Result<Response, ApiError> {
    toegang(&state, &claims)?;
    let tekst = match naam.as_str() {
        "staal-invoer" => constructie_core::staal_invoer_schema(),
        "beton-invoer" => constructie_core::beton_invoer_schema(),
        "hout-invoer" => constructie_core::hout_invoer_schema(),
        "resultaat" => constructie_core::resultaat_schema(),
        _ => return Err(ApiError::NotFound(format!("schema '{naam}' bestaat niet"))),
    };
    Ok(([(header::CONTENT_TYPE, "application/json")], tekst).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entitlements::DEFAULT_CONSTRUCTIE_GROUP;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    const S1: &str = r#"{"algemeen":{"overspanning_m":5.4,"permanent_kn_m2":0.75,"gevolgklasse":"CC2"},"belastingbreedte_m":3.6,"staalsoort":"S235","reeksen":["IPE","HEA"]}"#;

    const H1: &str = r#"{"algemeen":{"overspanning_m":4.2,"permanent_kn_m2":0.75,"eigen_gewicht_automatisch":false,"gevolgklasse":"CC2"},"element":{"type":"balklaag","hoh_mm":400},"sterkteklasse":"C24"}"#;

    async fn app() -> Router {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        let state = AppState::new(
            db,
            None,
            None,
            None,
            None,
            openaec_cloud::TenantsRegistry::default(),
            None,
        );
        constructie_routes().with_state(state)
    }

    fn verzoek(methode: &str, pad: &str, groepen: Option<&str>, body: &str) -> Request<Body> {
        let mut b = Request::builder().method(methode).uri(pad);
        if let Some(g) = groepen {
            b = b
                .header("X-Authentik-Username", "tester")
                .header("X-Authentik-Groups", g);
        }
        b.body(Body::from(body.to_string())).expect("request")
    }

    async fn antwoord(app: Router, req: Request<Body>) -> (StatusCode, Value) {
        let resp = app.oneshot(req).await.expect("response");
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 4 * 1024 * 1024)
            .await
            .expect("body");
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }

    #[tokio::test]
    async fn staal_s1_met_entitlement_geeft_ipe_200() {
        let (status, json) = antwoord(
            app().await,
            verzoek(
                "POST",
                "/constructie/staal",
                Some(DEFAULT_CONSTRUCTIE_GROUP),
                S1,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let advies = serde_json::to_string(&json["advies"]).expect("advies");
        assert!(advies.contains("IPE 200"), "advies: {advies}");
        assert!(json["disclaimer"].as_str().is_some_and(|d| !d.is_empty()));
    }

    #[tokio::test]
    async fn hout_h1_met_entitlement_geeft_200_met_advies() {
        let (status, json) = antwoord(
            app().await,
            verzoek(
                "POST",
                "/constructie/hout",
                Some(DEFAULT_CONSTRUCTIE_GROUP),
                H1,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["advies"]["naam"], "71 x 296");
        assert!(json["disclaimer"].as_str().is_some_and(|d| !d.is_empty()));
    }

    #[tokio::test]
    async fn hout_zonder_groep_403_en_onbekend_veld_422() {
        let (status, json) = antwoord(
            app().await,
            verzoek("POST", "/constructie/hout", Some("andere-groep"), H1),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"], GEEN_TOEGANG);
        let body = H1.replace("\"sterkteklasse\"", "\"onzin\":1,\"sterkteklasse\"");
        let (status, json) = antwoord(
            app().await,
            verzoek(
                "POST",
                "/constructie/hout",
                Some(DEFAULT_CONSTRUCTIE_GROUP),
                &body,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(json["detail"].as_str().is_some_and(|d| d.contains("onzin")));
    }

    #[tokio::test]
    async fn zonder_groep_403_met_letterlijke_melding() {
        let (status, json) = antwoord(
            app().await,
            verzoek("POST", "/constructie/staal", Some("andere-groep"), S1),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(json["error"], GEEN_TOEGANG);
    }

    #[tokio::test]
    async fn zonder_auth_401() {
        let (status, _) =
            antwoord(app().await, verzoek("POST", "/constructie/staal", None, S1)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn onbekend_veld_geeft_422() {
        let body = S1.replace("\"staalsoort\"", "\"onzin\":1,\"staalsoort\"");
        let (status, json) = antwoord(
            app().await,
            verzoek(
                "POST",
                "/constructie/staal",
                Some(DEFAULT_CONSTRUCTIE_GROUP),
                &body,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(json["error"].is_string());
        assert!(json["detail"].as_str().is_some_and(|d| d.contains("onzin")));
    }

    #[tokio::test]
    async fn schema_endpoint_en_onbekend_schema() {
        let (status, json) = antwoord(
            app().await,
            verzoek(
                "GET",
                "/constructie/schema/staal-invoer",
                Some(DEFAULT_CONSTRUCTIE_GROUP),
                "",
            ),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(json.get("properties").is_some());
        let (status, _) = antwoord(
            app().await,
            verzoek(
                "GET",
                "/constructie/schema/nope",
                Some(DEFAULT_CONSTRUCTIE_GROUP),
                "",
            ),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn rapport_bereikt_proxy_en_zonder_url_503() {
        let body = format!(r#"{{"materiaal":"staal","invoer":{S1}}}"#);
        let (status, _) = antwoord(
            app().await,
            verzoek(
                "POST",
                "/constructie/rapport",
                Some(DEFAULT_CONSTRUCTIE_GROUP),
                &body,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn rapport_neemt_geuploade_coverfoto_over_en_controleert_hem() {
        use base64::Engine;
        let claims = OidcClaims::default();
        // JPEG-signatuur gevolgd door opvulling tot `n` bytes.
        let b64 = |n: usize| {
            let mut bytes = vec![0xFF, 0xD8, 0xFF];
            bytes.resize(n.max(3), 0);
            base64::engine::general_purpose::STANDARD.encode(bytes)
        };
        let aanvraag = |foto: String| {
            format!(r#"{{"materiaal":"staal","invoer":{S1},"project":{{"coverfoto":{foto}}}}}"#)
        };

        let goed = aanvraag(format!(
            r#"{{"data":"{}","media_type":"image/jpeg","filename":"bouwplaats.jpg"}}"#,
            b64(10)
        ));
        let json = bouw_rapport(&goed, &claims).expect("rapport met foto");
        assert_eq!(json["cover"]["image"]["media_type"], "image/jpeg");
        assert_eq!(json["cover"]["image"]["filename"], "bouwplaats.jpg");
        assert_eq!(json["cover"]["image"]["data"], b64(10));
        assert!(json["cover"]["subtitle"].is_string(), "ondertitel blijft staan");

        // Zonder foto: geen cover.image (de proxy vult de standaard aan).
        let zonder = format!(r#"{{"materiaal":"staal","invoer":{S1}}}"#);
        assert!(bouw_rapport(&zonder, &claims).expect("rapport")["cover"]["image"].is_null());

        let gif = aanvraag(format!(r#"{{"data":"{}","media_type":"image/gif"}}"#, b64(10)));
        assert!(matches!(bouw_rapport(&gif, &claims), Err(ApiError::BadRequest(_))));
        let groot = aanvraag(format!(
            r#"{{"data":"{}","media_type":"image/jpeg"}}"#,
            b64(MAX_COVERFOTO_BYTES + 1)
        ));
        // JPEG-inhoud met PNG-type: signatuur klopt niet.
        let verkeerd = aanvraag(format!(r#"{{"data":"{}","media_type":"image/png"}}"#, b64(10)));
        assert!(matches!(bouw_rapport(&verkeerd, &claims), Err(ApiError::BadRequest(_))));
        assert!(matches!(bouw_rapport(&groot, &claims), Err(ApiError::BadRequest(_))));
        let kapot = aanvraag(r#"{"data":"!!","media_type":"image/png"}"#.to_string());
        assert!(matches!(bouw_rapport(&kapot, &claims), Err(ApiError::BadRequest(_))));
    }

    #[tokio::test]
    async fn rapportroute_accepteert_body_tot_4_mb() {
        // Zelfde laagopbouw als main.rs: module-laag 2 MB, route-laag 4 MB.
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        let state = AppState::new(db, None, None, None, None, Default::default(), None);
        let maak = || {
            constructie_routes()
                .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
                .with_state(state.clone())
        };
        let body = |bytes: usize| format!(r#"{{"x":"{}"}}"#, "a".repeat(bytes));
        // 3 MB komt door de limiet heen (en struikelt daarna over de onbekende sleutel).
        let (status, _) = antwoord(
            maak(),
            verzoek("POST", "/constructie/rapport", Some(DEFAULT_CONSTRUCTIE_GROUP), &body(3 << 20)),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        let (status, _) = antwoord(
            maak(),
            verzoek("POST", "/constructie/rapport", Some(DEFAULT_CONSTRUCTIE_GROUP), &body(5 << 20)),
        )
        .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        // Een reken-route houdt de 2 MB van de module.
        let (status, _) = antwoord(
            maak(),
            verzoek("POST", "/constructie/staal", Some(DEFAULT_CONSTRUCTIE_GROUP), &body(3 << 20)),
        )
        .await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn rapport_bevat_altijd_disclaimer() {
        let claims = OidcClaims::default();
        let body = format!(
            r#"{{"materiaal":"staal","invoer":{S1},"project":{{"naam":"Proef","opsteller":"A. B."}}}}"#
        );
        let json = bouw_rapport(&body, &claims).expect("rapport");
        assert_eq!(json["project"], "Proef");
        assert_eq!(json["author"], "A. B.");
        let tekst = json.to_string();
        assert!(tekst.contains(constructie_core::DISCLAIMER));
        // Lege hout-invoer en onbekende velden zijn invoerfouten.
        assert!(bouw_rapport(r#"{"materiaal":"hout","invoer":{}}"#, &claims).is_err());
        let hout = format!(r#"{{"materiaal":"hout","invoer":{H1}}}"#);
        let json = bouw_rapport(&hout, &claims).expect("hout-rapport");
        assert!(json.to_string().contains(constructie_core::DISCLAIMER));
        assert!(bouw_rapport(r#"{"materiaal":"staal","invoer":{},"x":1}"#, &claims).is_err());
    }
}
