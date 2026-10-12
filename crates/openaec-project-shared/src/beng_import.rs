//! Ontvangst van het contract **beng-import v1** — BENG-geometrie uit een
//! externe bron (de IFC-route van ifc-ruimtebalans) naar
//! [`crate::ProjectV2::beng_geometry`].
//!
//! ## Contract (afgesproken 12-10-2026, optie B)
//!
//! ```text
//! {
//!   "format": "beng-import", "version": "1.0",
//!   "source": "...", "exported_at": "ISO 8601",
//!   "project_name"?: "...", "true_north_deg"?: 0.0,
//!   "compleet": true|false,
//!   "beng_geometry": { BengGeometry, exact de serde-vorm van crate::beng_geometry },
//!   "herkomst"?: [], "zones_extra"?: [], "gevels_extra"?: [],
//!   "opaque_defs_extra"?: [], "window_defs_extra"?: [], "meldingen"?: [],
//!   "aansluitingen"?: [], "belemmering_geometrie"?: [],
//!   "vloeren_per_bouwlaag"?: [], "pv_vlakken"?: []
//! }
//! ```
//!
//! - De kern (`beng_geometry`) gaat zonder mapping het project in.
//! - De optionele blokken worden in v1 alleen op vorm gecontroleerd (array van
//!   objecten) en als `genegeerd` teruggemeld; `meldingen` gaan door naar de
//!   gebruiker.
//! - Onbekende sleutels zijn een fout, ook binnen `beng_geometry` (vangt
//!   typfouten zoals `frame_fractie`). Sleutels die met `_` beginnen gelden als
//!   commentaar en worden genegeerd.
//! - Zolang de IFC-route niet geijkt is (R-3), is de uitkomst "te controleren":
//!   `te_controleren = true` in het antwoord en `beng_geometry.bron` gezet, zodat
//!   ook de latere BENG-berekening dat meldt.
//! - Rc/U en U_w komen niet uit IFC. Bij `compleet = false` mogen
//!   `opaque_defs[].thermal` en `window_defs[].u_w_per_m2k` ontbreken; de
//!   import geeft dan status [`BengImportStatus::Onvolledig`] met de
//!   ontbrekende JSON-paden en GEEN geometrie. Er wordt niets ingevuld.
//!
//! Schema: `schemas/v1/beng-import.schema.json`, gegenereerd door
//! [`beng_import_schema_json`] (drift-test in `tests/beng_import_test.rs`).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::beng_geometry::BengGeometry;

/// Melding bij elke import zolang de IFC-route niet geijkt is (R-3).
pub const MELDING_TE_CONTROLEREN: &str = "Te controleren: BENG-geometrie uit de IFC-route is nog niet tegen certified berekeningen geijkt (R-3). Controleer de uitkomst voor gebruik.";

/// Vaste waarde van het `format`-veld.
pub const BENG_IMPORT_FORMAT: &str = "beng-import";
/// De enige versie die deze ontvanger accepteert.
pub const BENG_IMPORT_VERSION: &str = "1.0";

/// Optionele blokken die v1 accepteert maar niet leest.
pub const GENEGEERDE_BLOKKEN: [&str; 9] = [
    "herkomst",
    "zones_extra",
    "gevels_extra",
    "opaque_defs_extra",
    "window_defs_extra",
    "aansluitingen",
    "belemmering_geometrie",
    "vloeren_per_bouwlaag",
    "pv_vlakken",
];

/// Het beng-import-v1-document (typed vorm; alleen te deserialiseren als de
/// geometrie compleet is). Dient ook als bron voor het JSON-schema.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BengImportV1 {
    /// Altijd `"beng-import"`.
    pub format: String,
    /// Contractversie, `"1.0"`.
    pub version: String,
    /// Herkomst van het document (tool, versie).
    pub source: String,
    /// Exportmoment, ISO 8601.
    pub exported_at: String,
    /// Projectnaam (informatief).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Ware-noordhoek van het bronmodel in graden (informatief).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub true_north_deg: Option<f64>,
    /// `true` alleen als `beng_geometry` zonder aanvulling deserialiseert en
    /// valideert.
    pub compleet: bool,
    /// De kern: exact [`BengGeometry`].
    pub beng_geometry: BengGeometry,
    /// Herkomst per veld (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub herkomst: Vec<serde_json::Map<String, Value>>,
    /// Extra per rekenzone (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones_extra: Vec<serde_json::Map<String, Value>>,
    /// Extra per gevelvlak (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gevels_extra: Vec<serde_json::Map<String, Value>>,
    /// Extra per opake constructie, bv. lagen (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opaque_defs_extra: Vec<serde_json::Map<String, Value>>,
    /// Extra per kozijnmerk (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub window_defs_extra: Vec<serde_json::Map<String, Value>>,
    /// Meldingen van de bron; worden doorgegeven aan de gebruiker.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub meldingen: Vec<serde_json::Map<String, Value>>,
    /// Lineaire aansluitingen (type, lengte, zone) (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aansluitingen: Vec<serde_json::Map<String, Value>>,
    /// Belemmeringsgeometrie per raam (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub belemmering_geometrie: Vec<serde_json::Map<String, Value>>,
    /// Vloeren per bouwlaag met massaklasse (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vloeren_per_bouwlaag: Vec<serde_json::Map<String, Value>>,
    /// PV-vlakken (v1: niet gelezen).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pv_vlakken: Vec<serde_json::Map<String, Value>>,
}

/// Uitkomst van een geslaagde import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BengImportStatus {
    /// De geometrie is compleet en geldig; `beng_geometry` is gevuld.
    Compleet,
    /// Er ontbreken Rc/U- of U_w-waarden; geen geometrie, wel de paden.
    Onvolledig,
}

/// Resultaat van [`import_beng`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BengImportResult {
    /// Compleet of onvolledig.
    pub status: BengImportStatus,
    /// De geometrie voor `project.beng_geometry` (alleen bij compleet).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beng_geometry: Option<BengGeometry>,
    /// JSON-pointers naar ontbrekende waarden (alleen bij onvolledig).
    #[serde(default)]
    pub ontbrekend: Vec<String>,
    /// Meldingen voor de gebruiker (van de bron en van de import).
    #[serde(default)]
    pub meldingen: Vec<String>,
    /// Aanwezige optionele blokken die v1 niet leest.
    #[serde(default)]
    pub genegeerd: Vec<String>,
    /// De uitkomst is indicatief en moet gecontroleerd worden (de IFC-route is
    /// nog niet geijkt, R-3). Nu altijd `true`.
    pub te_controleren: bool,
    /// Projectnaam uit het document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Bron uit het document.
    pub source: String,
}

/// Fout bij de import: het document voldoet niet aan het contract. Alle
/// varianten zijn client-invoerfouten (422).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BengImportError {
    /// Geen JSON-object.
    #[error("geen JSON-object: {0}")]
    GeenObject(String),
    /// `format` of `version` klopt niet.
    #[error("{veld} moet \"{verwacht}\" zijn, kreeg {kreeg}")]
    Envelope {
        /// Het veld.
        veld: &'static str,
        /// De verwachte waarde.
        verwacht: &'static str,
        /// De ontvangen waarde.
        kreeg: String,
    },
    /// Structuur wijkt af van het schema (met JSON-pad).
    #[error("schemafout op {pad}: {reden}")]
    Schema {
        /// JSON-pad (punt-notatie) van de fout.
        pad: String,
        /// Reden van serde.
        reden: String,
    },
    /// `compleet = true` maar er ontbreken waarden.
    #[error("compleet = true, maar er ontbreken waarden: {0}")]
    StrijdigCompleet(String),
    /// De geometrie is structureel goed maar inhoudelijk ongeldig.
    #[error("ongeldige BENG-geometrie: {0}")]
    Validatie(String),
}

/// Importeer een beng-import-v1-document.
///
/// # Errors
/// [`BengImportError`] als het document niet aan het contract voldoet. Een
/// onvolledig maar verder correct document is GEEN fout: dat geeft
/// [`BengImportStatus::Onvolledig`].
pub fn import_beng(doc: &Value) -> Result<BengImportResult, BengImportError> {
    let obj = doc
        .as_object()
        .ok_or_else(|| BengImportError::GeenObject(type_naam(doc).into()))?;
    check_envelope(obj, "format", BENG_IMPORT_FORMAT)?;
    check_envelope(obj, "version", BENG_IMPORT_VERSION)?;

    let ontbrekend = ontbrekende_waarden(obj.get("beng_geometry"));
    let compleet_vlag = obj.get("compleet").and_then(Value::as_bool);

    if !ontbrekend.is_empty() {
        if compleet_vlag == Some(true) {
            return Err(BengImportError::StrijdigCompleet(ontbrekend.join(", ")));
        }
        // Vul de ontbrekende waarden NIET in; controleer de rest van de
        // structuur met een tijdelijke vervanger zodat schemafouten elders
        // niet verborgen blijven achter de onvolledigheid.
        let mut probe = doc.clone();
        vul_ontbrekend_voor_controle(&mut probe);
        let typed = parse_typed(&probe)?;
        // Ook de inhoud (referenties, oppervlakken) nu al toetsen; de vervanger
        // zelf (U = 1,0) doorstaat de validatie altijd.
        typed
            .beng_geometry
            .validate()
            .map_err(|e| BengImportError::Validatie(e.to_string()))?;
        let mut result = basis_resultaat(&typed, obj);
        result.status = BengImportStatus::Onvolledig;
        result.ontbrekend = ontbrekend;
        result
            .meldingen
            .extend(belemmering_meldingen(&typed.beng_geometry));
        return Ok(result);
    }

    let typed = parse_typed(doc)?;
    let mut typed = typed;
    typed.beng_geometry.bron = Some(format!("beng-import:{}", typed.source));
    typed
        .beng_geometry
        .validate()
        .map_err(|e| BengImportError::Validatie(e.to_string()))?;
    let mut result = basis_resultaat(&typed, obj);
    if !typed.compleet {
        result
            .meldingen
            .push("compleet = false opgegeven, maar de geometrie is volledig en geldig.".into());
    }
    result
        .meldingen
        .extend(belemmering_meldingen(&typed.beng_geometry));
    result.beng_geometry = Some(typed.beng_geometry);
    Ok(result)
}

fn type_naam(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "getal",
        Value::String(_) => "tekst",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn check_envelope(
    obj: &serde_json::Map<String, Value>,
    veld: &'static str,
    verwacht: &'static str,
) -> Result<(), BengImportError> {
    match obj.get(veld) {
        Some(Value::String(s)) if s == verwacht => Ok(()),
        other => Err(BengImportError::Envelope {
            veld,
            verwacht,
            kreeg: other.map_or_else(|| "niets".into(), ToString::to_string),
        }),
    }
}

fn parse_typed(doc: &Value) -> Result<BengImportV1, BengImportError> {
    let mut schoon = doc.clone();
    verwijder_commentaar(&mut schoon);
    let typed: BengImportV1 =
        serde_path_to_error::deserialize(&schoon).map_err(|e| BengImportError::Schema {
            pad: e.path().to_string(),
            reden: e.inner().to_string(),
        })?;
    // De geneste typen weigeren onbekende velden niet zelf (bestaande
    // projectbestanden blijven zo leesbaar). Hier wel: serialiseer terug en
    // zoek sleutels die serde heeft laten vallen.
    let terug = serde_json::to_value(&typed.beng_geometry).map_err(|e| BengImportError::Schema {
        pad: "beng_geometry".into(),
        reden: e.to_string(),
    })?;
    if let Some(invoer) = schoon.get("beng_geometry") {
        let mut onbekend = Vec::new();
        zoek_onbekend(invoer, &terug, "beng_geometry", &mut onbekend);
        if let Some(pad) = onbekend.first() {
            return Err(BengImportError::Schema {
                pad: pad.clone(),
                reden: format!("onbekend veld ({} in totaal)", onbekend.len()),
            });
        }
    }
    Ok(typed)
}

/// Verwijder sleutels die met `_` beginnen (commentaar) op elk niveau.
fn verwijder_commentaar(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.retain(|k, _| !k.starts_with('_'));
            m.values_mut().for_each(verwijder_commentaar);
        }
        Value::Array(a) => a.iter_mut().for_each(verwijder_commentaar),
        _ => {}
    }
}

/// Sleutels in `invoer` die na deserialiseren en terugserialiseren ontbreken
/// in `terug`. Een sleutel die alleen ontbreekt omdat hij de default had
/// (`null`, `false`, `""`, `[]`, `{}`, `"none"`) telt niet.
fn zoek_onbekend(invoer: &Value, terug: &Value, pad: &str, uit: &mut Vec<String>) {
    match (invoer, terug) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in a {
                let sub = format!("{pad}.{k}");
                match b.get(k) {
                    Some(t) => zoek_onbekend(v, t, &sub, uit),
                    None if is_default_waarde(v) => {}
                    None => uit.push(sub),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            for (i, (v, t)) in a.iter().zip(b).enumerate() {
                zoek_onbekend(v, t, &format!("{pad}[{i}]"), uit);
            }
        }
        _ => {}
    }
}

fn is_default_waarde(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::String(s) => s.is_empty() || s == "none",
        Value::Array(a) => a.is_empty(),
        Value::Object(m) => m.is_empty(),
        Value::Number(_) => false,
    }
}

fn basis_resultaat(typed: &BengImportV1, obj: &serde_json::Map<String, Value>) -> BengImportResult {
    let genegeerd = GENEGEERDE_BLOKKEN
        .iter()
        .filter(|k| {
            obj.get(**k)
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty())
        })
        .map(|k| (*k).to_string())
        .collect();
    let mut meldingen: Vec<String> = vec![MELDING_TE_CONTROLEREN.to_string()];
    meldingen.extend(typed.meldingen.iter().map(melding_tekst));
    BengImportResult {
        status: BengImportStatus::Compleet,
        beng_geometry: None,
        ontbrekend: Vec::new(),
        meldingen,
        genegeerd,
        te_controleren: true,
        project_name: typed.project_name.clone(),
        source: typed.source.clone(),
    }
}

/// Een bronmelding als één regel: `code: tekst` waar beschikbaar.
fn melding_tekst(m: &serde_json::Map<String, Value>) -> String {
    let code = m.get("code").and_then(Value::as_str);
    let tekst = m.get("tekst").and_then(Value::as_str);
    match (code, tekst) {
        (Some(c), Some(t)) => format!("{c}: {t}"),
        (None, Some(t)) => t.to_string(),
        _ => Value::Object(m.clone()).to_string(),
    }
}

/// JSON-pointers van de Rc/U- en U_w-waarden die bij `compleet = false`
/// mogen ontbreken.
fn ontbrekende_waarden(geo: Option<&Value>) -> Vec<String> {
    let mut out = Vec::new();
    let Some(geo) = geo else {
        return out;
    };
    let lijst = |key: &str| {
        geo.get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    for (i, d) in lijst("opaque_defs").iter().enumerate() {
        if d.is_object() && d.get("thermal").is_none() {
            out.push(format!("/beng_geometry/opaque_defs/{i}/thermal"));
        }
    }
    for (i, d) in lijst("window_defs").iter().enumerate() {
        if d.is_object() && d.get("u_w_per_m2k").is_none() {
            out.push(format!("/beng_geometry/window_defs/{i}/u_w_per_m2k"));
        }
    }
    out
}

/// Tijdelijke vervanger, alleen om de overige structuur te kunnen controleren.
/// Het resultaat van deze controle wordt nooit teruggegeven.
fn vul_ontbrekend_voor_controle(doc: &mut Value) {
    let Some(geo) = doc.get_mut("beng_geometry") else {
        return;
    };
    for (key, veld, waarde) in [
        ("opaque_defs", "thermal", serde_json::json!({"u": 1.0})),
        ("window_defs", "u_w_per_m2k", serde_json::json!(1.0)),
    ] {
        if let Some(arr) = geo.get_mut(key).and_then(Value::as_array_mut) {
            for d in arr.iter_mut().filter_map(Value::as_object_mut) {
                d.entry(veld).or_insert_with(|| waarde.clone());
            }
        }
    }
}

/// Een raam zonder belemmering rekent als "geen belemmering" (factor 1,0).
/// Dat is een keuze, geen onbekende: meld het zodat de gebruiker kiest.
fn belemmering_meldingen(geo: &BengGeometry) -> Vec<String> {
    let n: usize = geo
        .zones
        .iter()
        .flat_map(|z| z.gevels.iter())
        .flat_map(|g| g.ramen.iter())
        .filter(|r| r.belemmering.is_none())
        .count();
    if n == 0 {
        Vec::new()
    } else {
        vec![format!(
            "{n} raamplaatsing(en) zonder belemmering: gerekend als 'geen belemmering' \
             (F_sh;obst = 1,0). Kies per raam minimaal, overstek, volledig of overig \
             (NTA 8800 §17.3.2)."
        )]
    }
}

/// JSON-schema van het beng-import-v1-document (pretty, LF, met afsluitende
/// newline). Op basis van [`BengImportV1`]; daarna zijn `thermal` en
/// `u_w_per_m2k` uit de `required`-lijsten gehaald, omdat ze bij
/// `compleet = false` mogen ontbreken.
#[must_use]
pub fn beng_import_schema_json() -> String {
    let schema = schemars::schema_for!(BengImportV1);
    let mut v = serde_json::to_value(&schema).expect("schema serialiseert");
    if let Some(defs) = v.get_mut("definitions").and_then(Value::as_object_mut) {
        for (def, veld) in [
            ("OpaqueConstructionDef", "thermal"),
            ("WindowDef", "u_w_per_m2k"),
        ] {
            if let Some(req) = defs
                .get_mut(def)
                .and_then(|d| d.get_mut("required"))
                .and_then(Value::as_array_mut)
            {
                req.retain(|r| r.as_str() != Some(veld));
            }
        }
    }
    let mut s = serde_json::to_string_pretty(&v).expect("schema serialiseert");
    s.push('\n');
    s
}
