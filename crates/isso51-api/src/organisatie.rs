//! Organisaties: gebruiker -> organisatie -> huisstijl, uit een bron.
//!
//! Een gebruiker hoort bij een organisatie via een Authentik-groep met het
//! voorvoegsel [`GROEP_PREFIX`] (bijv. `org-kba`). De groepen komen binnen via
//! `X-Authentik-Groups` (forward-auth) of de `groups` uit Authentik `users/me`
//! (Bearer) en staan in [`OidcClaims::groups`].
//!
//! Per organisatie legt het configuratiebestand (env `ORGANISATIES_CONFIG`,
//! voorbeeld `organisaties.example.json` in de repo-root) vast: naam, groep,
//! toegestane huisstijlen, standaardhuisstijl, CORS-origins en (later) het
//! abonnement. Er is geen tweede lijst: CORS wordt hieruit afgeleid.
//!
//! Regels:
//! - 0 groepen `org-*` -> geen organisatie (rekenen en projecten werken,
//!   rapporten niet).
//! - Precies 1, en die staat in de configuratie -> die organisatie.
//! - 1 die niet in de configuratie staat -> fout `onbekende_organisatie`.
//! - Meer dan 1 -> fout `meerdere_organisaties`. Geen gok, geen fallback.
//!
//! Een ongeldig of onleesbaar configuratiebestand laat de API niet starten.
//! Zie `docs/2026-10-09-organisaties.md` voor het toevoegen van een organisatie.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::auth::OidcClaims;

/// Voorvoegsel van Authentik-groepen die een organisatie aanwijzen.
pub const GROEP_PREFIX: &str = "org-";

/// Env-variabele met het pad naar het configuratiebestand.
pub const ENV_ORGANISATIES_CONFIG: &str = "ORGANISATIES_CONFIG";

/// Maximale lengte van een id of huisstijlnaam.
const MAX_SLUG_LEN: usize = 64;

/// Een organisatie zoals in het configuratiebestand.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Organisatie {
    /// Id (sleutel in het bestand), ook de tenant voor de Reports API en de
    /// bestandsnaam van de standaard-coverfoto. Wordt bij het laden gevuld.
    #[serde(skip_deserializing)]
    pub id: String,
    /// Weergavenaam, o.a. auteur/adviseur in rapporten.
    pub naam: String,
    /// Authentik-groep die het lidmaatschap bepaalt (moet met `org-` beginnen).
    pub groep: String,
    /// Huisstijlen (brands in de Reports API) die de organisatie mag kiezen.
    pub huisstijlen: Vec<String>,
    /// Huisstijl als het rapport er geen kiest; moet in `huisstijlen` staan.
    pub standaard_huisstijl: String,
    /// Origins die cross-origin met credentials de API mogen aanroepen.
    #[serde(default)]
    pub cors_origins: Vec<String>,
    /// Gereserveerd voor licentie/abonnement; nog zonder betekenis.
    #[serde(default)]
    pub abonnement: Option<serde_json::Value>,
}

impl Organisatie {
    /// Mag deze organisatie `huisstijl` gebruiken?
    pub fn mag_huisstijl(&self, huisstijl: &str) -> bool {
        self.huisstijlen.iter().any(|h| h == huisstijl)
    }
}

/// Wat `/me` en de frontend van een organisatie te zien krijgen.
#[derive(Clone, Debug, Serialize)]
pub struct OrganisatieProfiel {
    pub id: String,
    pub naam: String,
    pub huisstijlen: Vec<String>,
    pub standaard_huisstijl: String,
}

impl From<&Organisatie> for OrganisatieProfiel {
    fn from(o: &Organisatie) -> Self {
        Self {
            id: o.id.clone(),
            naam: o.naam.clone(),
            huisstijlen: o.huisstijlen.clone(),
            standaard_huisstijl: o.standaard_huisstijl.clone(),
        }
    }
}

/// Fout bij het afleiden van de organisatie uit de groepen.
#[derive(Clone, Debug, PartialEq)]
pub enum OrganisatieFout {
    /// Lid van meer dan een `org-*`-groep.
    Meerdere(Vec<String>),
    /// Lid van een `org-*`-groep die niet in de configuratie staat.
    Onbekend(String),
}

impl OrganisatieFout {
    /// Machineleesbare code (`error` in de API-respons).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Meerdere(_) => "meerdere_organisaties",
            Self::Onbekend(_) => "onbekende_organisatie",
        }
    }

    /// Leesbare toelichting voor de gebruiker.
    pub fn detail(&self) -> String {
        match self {
            Self::Meerdere(groepen) => format!(
                "Je account hoort bij meer dan een organisatie ({}); vraag de beheerder dit recht te zetten",
                groepen.join(", ")
            ),
            Self::Onbekend(groep) => format!(
                "Je account hoort bij organisatiegroep '{groep}', maar die organisatie is niet ingericht; vraag de beheerder"
            ),
        }
    }
}

/// Bestandsvorm: `{ "organisaties": { "<id>": { ... } } }`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bestand {
    organisaties: BTreeMap<String, Organisatie>,
}

/// Alle organisaties, gevalideerd.
#[derive(Clone, Debug, Default)]
pub struct Organisaties {
    lijst: BTreeMap<String, Organisatie>,
}

impl Organisaties {
    /// Lees en valideer het configuratiebestand.
    pub fn laad(pad: &Path) -> Result<Self, String> {
        let tekst = std::fs::read_to_string(pad)
            .map_err(|e| format!("{}: niet leesbaar: {e}", pad.display()))?;
        Self::uit_json(&tekst).map_err(|e| format!("{}: {e}", pad.display()))
    }

    /// Parse en valideer de inhoud van het configuratiebestand.
    pub fn uit_json(tekst: &str) -> Result<Self, String> {
        let bestand: Bestand =
            serde_json::from_str(tekst).map_err(|e| format!("ongeldige JSON: {e}"))?;
        let mut groepen = HashSet::new();
        let mut lijst = BTreeMap::new();
        for (id, mut org) in bestand.organisaties {
            if !is_slug(&id) {
                return Err(format!("organisatie-id '{id}': alleen a-z, 0-9, - en _"));
            }
            org.id = id.clone();
            valideer(&org)?;
            if !groepen.insert(org.groep.clone()) {
                return Err(format!(
                    "organisatie '{id}': groep '{}' staat bij meer dan een organisatie",
                    org.groep
                ));
            }
            lijst.insert(id, org);
        }
        Ok(Self { lijst })
    }

    /// Aantal organisaties.
    pub fn len(&self) -> usize {
        self.lijst.len()
    }

    /// Geen enkele organisatie ingericht?
    pub fn is_empty(&self) -> bool {
        self.lijst.is_empty()
    }

    /// Ids van alle organisaties (gesorteerd).
    pub fn ids(&self) -> Vec<&str> {
        self.lijst.keys().map(String::as_str).collect()
    }

    /// Leid de organisatie af uit de groepen van de gebruiker.
    pub fn van_groepen(&self, groepen: &[String]) -> Result<Option<&Organisatie>, OrganisatieFout> {
        let mut org_groepen: Vec<&str> = groepen
            .iter()
            .map(String::as_str)
            .filter(|g| g.starts_with(GROEP_PREFIX))
            .collect();
        org_groepen.sort_unstable();
        org_groepen.dedup();
        match org_groepen.as_slice() {
            [] => Ok(None),
            [groep] => self
                .lijst
                .values()
                .find(|o| o.groep == *groep)
                .map(Some)
                .ok_or_else(|| OrganisatieFout::Onbekend((*groep).to_string())),
            meer => Err(OrganisatieFout::Meerdere(
                meer.iter().map(|g| (*g).to_string()).collect(),
            )),
        }
    }

    /// Organisatie van een geauthenticeerde gebruiker.
    pub fn van_claims(&self, claims: &OidcClaims) -> Result<Option<&Organisatie>, OrganisatieFout> {
        self.van_groepen(&claims.groups)
    }

    /// Vereniging van de CORS-origins van alle organisaties.
    pub fn cors_origins(&self) -> HashSet<String> {
        self.lijst
            .values()
            .flat_map(|o| o.cors_origins.iter().cloned())
            .collect()
    }
}

fn valideer(org: &Organisatie) -> Result<(), String> {
    let id = &org.id;
    if org.naam.trim().is_empty() {
        return Err(format!("organisatie '{id}': naam is leeg"));
    }
    if !org.groep.starts_with(GROEP_PREFIX) || org.groep.len() == GROEP_PREFIX.len() {
        return Err(format!(
            "organisatie '{id}': groep '{}' moet met '{GROEP_PREFIX}' beginnen",
            org.groep
        ));
    }
    if org.huisstijlen.is_empty() {
        return Err(format!("organisatie '{id}': huisstijlen is leeg"));
    }
    let mut gezien = HashSet::new();
    for h in &org.huisstijlen {
        if !is_slug(h) {
            return Err(format!("organisatie '{id}': huisstijl '{h}': alleen a-z, 0-9, - en _"));
        }
        if !gezien.insert(h) {
            return Err(format!("organisatie '{id}': huisstijl '{h}' staat dubbel"));
        }
    }
    if !org.mag_huisstijl(&org.standaard_huisstijl) {
        return Err(format!(
            "organisatie '{id}': standaard_huisstijl '{}' staat niet in huisstijlen",
            org.standaard_huisstijl
        ));
    }
    for origin in &org.cors_origins {
        if !crate::cors::is_valid_origin(origin) {
            return Err(format!(
                "organisatie '{id}': cors-origin '{origin}' ongeldig (http(s)://, kleine letters, geen / aan het eind, geen *)"
            ));
        }
    }
    Ok(())
}

/// Testconfiguratie: organisatie `kba` (groep `org-kba`) met huisstijlen
/// `kba` (standaard) en `openaec_foundation`.
#[cfg(test)]
pub fn test_organisaties() -> Organisaties {
    Organisaties::uit_json(
        r#"{"organisaties":{"kba":{"naam":"Testbureau","groep":"org-kba",
            "huisstijlen":["kba","openaec_foundation"],"standaard_huisstijl":"kba"}}}"#,
    )
    .expect("testconfiguratie")
}

/// Alleen `[a-z0-9_-]`, 1..=64 tekens: ids en huisstijlen worden bestandsnamen.
pub fn is_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_SLUG_LEN
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    const GELDIG: &str = r#"{ "organisaties": {
        "kba": { "naam": "Voorbeeld Bouwadvies", "groep": "org-kba",
                 "huisstijlen": ["kba", "openaec_foundation"], "standaard_huisstijl": "kba",
                 "cors_origins": ["https://app.example.org"] },
        "andere": { "naam": "Andere", "groep": "org-andere",
                    "huisstijlen": ["andere"], "standaard_huisstijl": "andere" } } }"#;

    fn groepen(g: &[&str]) -> Vec<String> {
        g.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn voorbeeldbestand_in_repo_is_geldig() {
        let tekst = include_str!("../../../organisaties.example.json");
        let orgs = Organisaties::uit_json(tekst).expect("voorbeeld geldig");
        assert!(!orgs.is_empty());
    }

    #[test]
    fn laadt_geldige_configuratie() {
        let orgs = Organisaties::uit_json(GELDIG).expect("geldig");
        assert_eq!(orgs.ids(), vec!["andere", "kba"]);
        let kba = orgs.van_groepen(&groepen(&["org-kba"])).unwrap().unwrap();
        assert_eq!(kba.id, "kba");
        assert_eq!(kba.standaard_huisstijl, "kba");
        assert!(kba.mag_huisstijl("openaec_foundation"));
        assert!(!kba.mag_huisstijl("3bm"));
        assert_eq!(
            orgs.cors_origins(),
            HashSet::from(["https://app.example.org".to_string()])
        );
    }

    #[test]
    fn organisatie_uit_groepen() {
        let orgs = Organisaties::uit_json(GELDIG).unwrap();
        // Geen org-groep: geen organisatie; andere groepen tellen niet.
        assert_eq!(orgs.van_groepen(&groepen(&[])), Ok(None));
        assert_eq!(
            orgs.van_groepen(&groepen(&["openbouwlab-constructie", "admins"])),
            Ok(None)
        );
        // Precies een (dubbel in de header telt als een).
        let o = orgs
            .van_groepen(&groepen(&["admins", "org-kba", "org-kba"]))
            .unwrap()
            .unwrap();
        assert_eq!(o.id, "kba");
        // Meer dan een: fout, geen gok.
        let fout = orgs.van_groepen(&groepen(&["org-kba", "org-andere"])).unwrap_err();
        assert_eq!(fout.code(), "meerdere_organisaties");
        assert_eq!(fout, OrganisatieFout::Meerdere(groepen(&["org-andere", "org-kba"])));
        // Onbekende org-groep: fout, geen fallback.
        let fout = orgs.van_groepen(&groepen(&["org-onbekend"])).unwrap_err();
        assert_eq!(fout, OrganisatieFout::Onbekend("org-onbekend".to_string()));
        assert!(fout.detail().contains("org-onbekend"));
        // Exacte match: hoofdletters zijn geen org-groep van deze lijst.
        assert!(orgs.van_groepen(&groepen(&["Org-kba"])).unwrap().is_none());
    }

    #[test]
    fn lege_configuratie_kent_niemand() {
        let orgs = Organisaties::default();
        assert_eq!(orgs.van_groepen(&groepen(&["admins"])), Ok(None));
        assert!(matches!(
            orgs.van_groepen(&groepen(&["org-kba"])),
            Err(OrganisatieFout::Onbekend(_))
        ));
    }

    #[test]
    fn ongeldige_configuraties_worden_geweigerd() {
        let fout = |json: &str| Organisaties::uit_json(json).unwrap_err();
        let org = |veld: &str| {
            format!(
                r#"{{"organisaties":{{"kba":{{"naam":"K","groep":"org-kba","huisstijlen":["kba"],"standaard_huisstijl":"kba"{veld}}}}}}}"#
            )
        };
        // Standaard buiten de lijst.
        assert!(fout(r#"{"organisaties":{"kba":{"naam":"K","groep":"org-kba","huisstijlen":["kba"],"standaard_huisstijl":"3bm"}}}"#)
            .contains("standaard_huisstijl"));
        // Groep zonder org-prefix.
        assert!(fout(r#"{"organisaties":{"kba":{"naam":"K","groep":"kba","huisstijlen":["kba"],"standaard_huisstijl":"kba"}}}"#)
            .contains("org-"));
        // Ongeldig id.
        assert!(fout(r#"{"organisaties":{"KBA":{"naam":"K","groep":"org-kba","huisstijlen":["kba"],"standaard_huisstijl":"kba"}}}"#)
            .contains("organisatie-id"));
        // Lege huisstijlen, lege naam.
        assert!(fout(r#"{"organisaties":{"kba":{"naam":"K","groep":"org-kba","huisstijlen":[],"standaard_huisstijl":"kba"}}}"#)
            .contains("huisstijlen is leeg"));
        assert!(fout(r#"{"organisaties":{"kba":{"naam":" ","groep":"org-kba","huisstijlen":["kba"],"standaard_huisstijl":"kba"}}}"#)
            .contains("naam is leeg"));
        // Dezelfde groep bij twee organisaties.
        assert!(fout(r#"{"organisaties":{
            "a":{"naam":"A","groep":"org-x","huisstijlen":["a"],"standaard_huisstijl":"a"},
            "b":{"naam":"B","groep":"org-x","huisstijlen":["b"],"standaard_huisstijl":"b"}}}"#)
            .contains("meer dan een organisatie"));
        // Ongeldige CORS-origin, onbekend veld (typfout), geen JSON.
        assert!(fout(&org(r#","cors_origins":["https://x.nl/"]"#)).contains("cors-origin"));
        assert!(fout(&org(r#","huistijlen":["kba"]"#)).contains("ongeldige JSON"));
        assert!(fout("geen json").contains("ongeldige JSON"));
        // Abonnement is gereserveerd en mag al gevuld zijn.
        assert!(Organisaties::uit_json(&org(r#","abonnement":{"plan":"x"}"#)).is_ok());
    }

    #[test]
    fn laad_meldt_pad_bij_ontbrekend_of_ongeldig_bestand() {
        let dir = std::env::temp_dir().join(format!("isso51-org-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("map");
        let ontbreekt = dir.join("bestaat-niet.json");
        assert!(Organisaties::laad(&ontbreekt).unwrap_err().contains("niet leesbaar"));
        let ongeldig = dir.join("ongeldig.json");
        std::fs::write(&ongeldig, r#"{"organisaties":{"kba":{"naam":"K"}}}"#).expect("schrijf");
        let fout = Organisaties::laad(&ongeldig).unwrap_err();
        assert!(fout.contains("ongeldig.json") && fout.contains("ongeldige JSON"), "{fout}");
    }

    #[test]
    fn slug_regels() {
        assert!(is_slug("kba"));
        assert!(is_slug("openaec_foundation"));
        assert!(!is_slug(""));
        assert!(!is_slug("../kba"));
        assert!(!is_slug("Kba"));
        assert!(!is_slug(&"a".repeat(65)));
    }
}
