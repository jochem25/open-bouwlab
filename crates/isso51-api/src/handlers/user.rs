//! User profile handler with OIDC-based upsert.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::auth::AuthClaims;
use crate::error::ApiError;
use crate::organisatie::OrganisatieProfiel;
use crate::state::AppState;

/// User profile response.
#[derive(Serialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub name: String,
    pub preferred_username: String,
    pub first_seen_at: String,
    pub last_login_at: String,
    /// Actieve module-entitlements (bv. `"constructie"`). Additief veld.
    #[serde(default)]
    pub entitlements: Vec<String>,
    /// Organisatie uit de `org-*`-groep; `null` zonder (geldige) organisatie.
    pub organisatie: Option<OrganisatieProfiel>,
    /// Waarom er geen organisatie is bij een `org-*`-groep die niet klopt
    /// (meerdere of onbekend); `null` als er niets mis is.
    pub organisatie_fout: Option<OrganisatieFoutProfiel>,
}

/// Fout bij het afleiden van de organisatie, voor de UI.
#[derive(Serialize)]
pub struct OrganisatieFoutProfiel {
    pub code: &'static str,
    pub detail: String,
}

/// GET /me — Return the current user's profile, creating it if it doesn't exist.
///
/// Uses OIDC claims from the JWT token to upsert the user record.
pub async fn get_profile(
    State(state): State<AppState>,
    AuthClaims(claims): AuthClaims,
) -> Result<Json<UserProfile>, ApiError> {
    let sub = &claims.sub;
    let email = claims.email.as_deref().unwrap_or("");
    let name = claims.name.as_deref().unwrap_or("");
    let preferred_username = claims.preferred_username.as_deref().unwrap_or("");
    let issuer = claims.iss.as_deref().unwrap_or("");

    // Upsert: insert if new, update last_login + profile fields if existing.
    sqlx::query(
        "INSERT INTO users (id, email, name, preferred_username, oidc_issuer)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET
           email = excluded.email,
           name = excluded.name,
           preferred_username = excluded.preferred_username,
           last_login_at = datetime('now')",
    )
    .bind(sub)
    .bind(email)
    .bind(name)
    .bind(preferred_username)
    .bind(issuer)
    .execute(&state.db)
    .await?;

    // Fetch the full profile back.
    let row = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, name, preferred_username, first_seen_at, last_login_at
         FROM users WHERE id = ?1",
    )
    .bind(sub)
    .fetch_one(&state.db)
    .await?;

    let (organisatie, organisatie_fout) = match state.organisaties.van_claims(&claims) {
        Ok(org) => (org.map(OrganisatieProfiel::from), None),
        Err(fout) => (
            None,
            Some(OrganisatieFoutProfiel {
                code: fout.code(),
                detail: fout.detail(),
            }),
        ),
    };

    Ok(Json(UserProfile {
        id: row.id,
        email: row.email,
        name: row.name,
        preferred_username: row.preferred_username,
        first_seen_at: row.first_seen_at,
        last_login_at: row.last_login_at,
        entitlements: crate::entitlements::entitlements_van(&claims, &state.entitlements),
        organisatie,
        organisatie_fout,
    }))
}

/// Internal row type for SQLx query mapping.
#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    email: String,
    name: String,
    preferred_username: String,
    first_seen_at: String,
    last_login_at: String,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;

    async fn me(groepen: &str) -> Value {
        let db = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("in-memory sqlite");
        crate::run_migrations(&db).await;
        let mut state = AppState::new(db, None, None, None);
        state.organisaties = Arc::new(crate::organisatie::test_organisaties());
        let app = Router::new().route("/me", get(get_profile)).with_state(state);
        let req = Request::builder()
            .uri("/me")
            .header("X-Authentik-Username", "tester")
            .header("X-Authentik-Groups", groepen)
            .body(Body::empty())
            .expect("request");
        let resp = app.oneshot(req).await.expect("response");
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.expect("body");
        serde_json::from_slice(&bytes).expect("json")
    }

    #[tokio::test]
    async fn me_geeft_organisatie_uit_org_groep() {
        let json = me("openbouwlab-constructie|org-kba").await;
        assert_eq!(json["organisatie"]["id"], "kba");
        assert_eq!(json["organisatie"]["naam"], "Testbureau");
        assert_eq!(json["organisatie"]["standaard_huisstijl"], "kba");
        assert_eq!(
            json["organisatie"]["huisstijlen"],
            serde_json::json!(["kba", "openaec_foundation"])
        );
        assert!(json["organisatie_fout"].is_null());
        // De groep (cors_origins, groep, abonnement) lekt niet naar de client.
        assert!(json["organisatie"].get("groep").is_none());
    }

    #[tokio::test]
    async fn me_zonder_of_met_foute_org_groep() {
        let json = me("openbouwlab-constructie").await;
        assert!(json["organisatie"].is_null());
        assert!(json["organisatie_fout"].is_null());

        let json = me("org-kba|org-andere").await;
        assert!(json["organisatie"].is_null());
        assert_eq!(json["organisatie_fout"]["code"], "meerdere_organisaties");

        let json = me("org-onbekend").await;
        assert_eq!(json["organisatie_fout"]["code"], "onbekende_organisatie");
    }
}
