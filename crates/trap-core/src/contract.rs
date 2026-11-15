//! Assemblagecontract `kba-assemblage/1` voor de trap (O-2026-11-15-05, S1).
//!
//! Contract: `%KBA_SHARED%/conventies/assemblagecontract/contract.md`. Deze
//! module bouwt de regels na die daar in Python (referentie) staan:
//!
//! - G: `GlobalId = IFC-base64(uuid5(NAMESPACE, "<project>|<onderdeel>|<element>"))`,
//!   delen genormaliseerd (NFC, witruimte samengevoegd, kleine letters);
//!   gouden vectoren gelijk aan `ref/tests/test_contract_guid.py`.
//! - O: ruimtelijke structuur en RD/NAP uit `project.json` (zelfde schema als
//!   `schema/projectoorsprong.schema.json`).
//! - Headless invoer (`ContractInvoer`): projectverwijzing, bouwlaag, houtsoort
//!   en het trapdeel van de bestaande invoer. Niveaus en projectnamen komen uit
//!   het project; de kern rekent daarna ongewijzigd (`bouw::ontwerp`).
//!
//! Puur: geen I/O. De CLI leest bestanden en geeft de inhoud door.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha1::{Digest as _, Sha1};
use unicode_normalization::UnicodeNormalization as _;

use crate::invoer::Invoer;
use crate::model::P2;
use crate::Fout;

pub const CONTRACT_VERSIE: &str = "kba-assemblage/1";
pub const INVOER_SCHEMA: &str = "kba-trap-headless/1";
pub const PROJECT_ONDERDEEL: &str = "_project";
/// uuid5(NAMESPACE_URL, "https://kolthofbouwadvies.nl/assemblagecontract/1"),
/// zie `contract_guid.py`; getoetst in `namespace_gelijk_aan_python`.
const NAMESPACE_URL: [u8; 16] = [
    0x6b, 0xa7, 0xb8, 0x11, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
];
const NAMESPACE_NAAM: &str = "https://kolthofbouwadvies.nl/assemblagecontract/1";

// ------------------------------------------------------------------ GUID-regel

/// Normaliseer een sleuteldeel; weigert lege delen en '|'.
pub fn normaliseer(deel: &str) -> Result<String, Fout> {
    let nfc: String = deel.nfc().collect();
    let s = nfc.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    if s.is_empty() {
        return Err(Fout::Invoer("leeg sleuteldeel (GUID-regel)".into()));
    }
    if s.contains('|') {
        return Err(Fout::Invoer(format!("'|' is verboden in een sleuteldeel: {deel:?}")));
    }
    Ok(s)
}

fn uuid5(namespace: &[u8; 16], naam: &str) -> [u8; 16] {
    let mut h = Sha1::new();
    h.update(namespace);
    h.update(naam.as_bytes());
    let d = h.finalize();
    let mut b = [0u8; 16];
    b.copy_from_slice(&d[..16]);
    b[6] = (b[6] & 0x0f) | 0x50;
    b[8] = (b[8] & 0x3f) | 0x80;
    b
}

fn namespace() -> [u8; 16] {
    uuid5(&NAMESPACE_URL, NAMESPACE_NAAM)
}

/// De genormaliseerde naamstring die de uuid5 ingaat.
pub fn sleutel(project: &str, onderdeel: &str, element: &str) -> Result<String, Fout> {
    Ok([normaliseer(project)?, normaliseer(onderdeel)?, normaliseer(element)?].join("|"))
}

pub fn contract_uuid(project: &str, onderdeel: &str, element: &str) -> Result<u128, Fout> {
    Ok(u128::from_be_bytes(uuid5(&namespace(), &sleutel(project, onderdeel, element)?)))
}

/// IfcGloballyUniqueId (22 tekens) volgens de contractregel.
pub fn contract_guid(project: &str, onderdeel: &str, element: &str) -> Result<String, Fout> {
    Ok(crate::p21::guid_uit_bits(contract_uuid(project, onderdeel, element)?))
}

// ------------------------------------------------------------------ project.json

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectOorsprong {
    /// Moet `kba-assemblage/1` zijn.
    pub schema: String,
    pub project: ProjectId,
    pub oorsprong: Oorsprong,
    pub bouwlagen: Vec<Bouwlaag>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectId {
    pub id: String,
    pub naam: String,
    #[serde(default = "standaard_gebouw")]
    pub gebouw: String,
    #[serde(default = "standaard_site")]
    pub site: String,
}

fn standaard_gebouw() -> String {
    "gebouw".into()
}
fn standaard_site() -> String {
    "kavel".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Oorsprong {
    pub rd_x: f64,
    pub rd_y: f64,
    pub nap_peil: f64,
    pub rotatie_ware_noorden: f64,
    #[serde(default)]
    pub crs: Option<String>,
    #[serde(default)]
    pub hoogtestelsel: Option<String>,
    #[serde(default)]
    pub bron: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bouwlaag {
    pub code: String,
    pub naam: String,
    pub peil: f64,
    #[serde(default)]
    pub vloerpakket: Option<f64>,
}

impl ProjectOorsprong {
    /// Toets wat het JSON-schema van het contract eist (patronen, consts, uniek).
    pub fn valideer(&self) -> Result<(), Fout> {
        let fout = |s: String| Err(Fout::Invoer(format!("project.json: {s}")));
        if self.schema != CONTRACT_VERSIE {
            return fout(format!("schema '{}', verwacht '{CONTRACT_VERSIE}'", self.schema));
        }
        let id = &self.project.id;
        let mut c = id.chars();
        if !c.next().is_some_and(|k| k.is_ascii_alphanumeric())
            || !c.all(|k| k.is_ascii_alphanumeric() || matches!(k, '.' | '_' | '-'))
        {
            return fout(format!("project.id '{id}' voldoet niet aan ^[A-Za-z0-9][A-Za-z0-9._-]*$"));
        }
        if self.project.naam.trim().is_empty() {
            return fout("project.naam is leeg".into());
        }
        if self.oorsprong.crs.as_deref().is_some_and(|s| s != "EPSG:28992") {
            return fout("oorsprong.crs moet EPSG:28992 zijn".into());
        }
        if self.oorsprong.hoogtestelsel.as_deref().is_some_and(|s| s != "NAP") {
            return fout("oorsprong.hoogtestelsel moet NAP zijn".into());
        }
        if !(-180.0..=180.0).contains(&self.oorsprong.rotatie_ware_noorden) {
            return fout("rotatie_ware_noorden buiten -180..180".into());
        }
        if self.bouwlagen.is_empty() {
            return fout("geen bouwlagen".into());
        }
        let mut codes = Vec::new();
        for b in &self.bouwlagen {
            if !laagcode_geldig(&b.code) {
                return fout(format!("bouwlaagcode '{}' voldoet niet aan ^-?[0-9]{{2}}[a-z]?$", b.code));
            }
            if b.vloerpakket.is_some_and(|v| v < 0.0) {
                return fout(format!("bouwlaag {}: vloerpakket negatief", b.code));
            }
            if codes.contains(&b.code.as_str()) {
                return fout(format!("dubbele bouwlaagcode '{}'", b.code));
            }
            codes.push(&b.code);
        }
        Ok(())
    }

    pub fn bouwlaag(&self, code: &str) -> Result<&Bouwlaag, Fout> {
        self.bouwlagen
            .iter()
            .find(|b| b.code == code)
            .ok_or_else(|| Fout::Invoer(format!("bouwlaag '{code}' staat niet in project.json")))
    }

    /// De eerstvolgende bouwlaag boven `code` (op peil).
    pub fn bouwlaag_erboven(&self, code: &str) -> Result<&Bouwlaag, Fout> {
        let peil = self.bouwlaag(code)?.peil;
        self.bouwlagen
            .iter()
            .filter(|b| b.peil > peil)
            .min_by(|a, b| a.peil.total_cmp(&b.peil))
            .ok_or_else(|| Fout::Invoer(format!("geen bouwlaag boven '{code}' in project.json")))
    }
}

fn laagcode_geldig(c: &str) -> bool {
    let b = c.strip_prefix('-').unwrap_or(c).as_bytes();
    matches!(b.len(), 2 | 3)
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && (b.len() == 2 || b[2].is_ascii_lowercase())
}

// ------------------------------------------------------------------ headless invoer

/// Houtsoort; wordt het NAA.K.T.-materiaal `hout_<soort>_generiek` (NAA.K.T.
/// v2.5, hoofdgroep hout) op de trap en elk deel. Lijst = de kenmerken uit
/// `IDS_NAAKT-v2.5.ids` die voor een houten trap in aanmerking komen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Houtsoort {
    Vuren,
    Grenen,
    Douglas,
    Lariks,
    Eiken,
    Beuken,
    Essen,
    Esdoorn,
    Meranti,
    Merbau,
    Accoya,
    Hardhout,
    Zachthout,
    Multiplex,
    Mdf,
}

impl Houtsoort {
    pub fn naakt(self) -> String {
        let s = serde_json::to_value(self).ok().and_then(|v| v.as_str().map(String::from));
        format!("hout_{}_generiek", s.unwrap_or_default())
    }
}

/// NL-SfB tabel 1 (BNA1991), viercijferig zonder haken.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NlSfb {
    pub code: String,
    pub omschrijving: String,
}

/// Projectbestand: pad (relatief aan het invoerbestand, de CLI leest het) of inline.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ProjectBron {
    Pad(String),
    Inline(Box<ProjectOorsprong>),
}

/// Headless invoer van de trapontwerper volgens het assemblagecontract.
///
/// `trap` is de bestaande trapinvoer (`schema/trap-invoer.schema.json`) ZONDER
/// `naam`, `project`, `niveaus` en materiaalvelden: die volgen uit het contract
/// (niveaus = peil van `bouwlaag` en `doorbreekt`, vloerdikte = vloerpakket van
/// `doorbreekt`, materiaal = `houtsoort`). Staan ze er toch in, dan weigert de
/// kern (geen twee bronnen voor een maat).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractInvoer {
    /// Moet `kba-trap-headless/1` zijn.
    pub schema: String,
    /// Onderdeel-id (`^[a-z0-9][a-z0-9-]*$`), tweede deel van elke GUID-sleutel.
    /// Nooit wijzigen of hergebruiken.
    pub onderdeel: String,
    /// Naam van de trap (IfcStair.Name); weg = onderdeel-id. Mag wijzigen
    /// zonder GUID-wijziging.
    #[serde(default)]
    pub naam: Option<String>,
    /// project.json (schema kba-assemblage/1): pad of inline.
    pub project: ProjectBron,
    /// Bouwlaagcode waarop de trap staat (aantrede 1).
    pub bouwlaag: String,
    /// Bouwlaagcode waarvan de vloer doorbroken wordt (bovenkant trap); weg =
    /// de eerstvolgende bouwlaag erboven.
    #[serde(default)]
    pub doorbreekt: Option<String>,
    pub houtsoort: Houtsoort,
    /// Weg = 24.11 (rechte steektrap) of 24.12 (niet-recht), uit de trapvorm.
    #[serde(default)]
    pub nlsfb: Option<NlSfb>,
    /// Trapdeel van de bestaande invoer (plattegrond in projectcoordinaten,
    /// trapgat, looplijn, verdrijving, ...); zie de typebeschrijving.
    #[schemars(with = "serde_json::Map<String, serde_json::Value>")]
    pub trap: serde_json::Map<String, serde_json::Value>,
}

/// Wat het contract aan de IFC-schrijver doorgeeft.
#[derive(Debug, Clone, Serialize)]
pub struct ContractIfc {
    pub project: ProjectOorsprong,
    pub laag: Bouwlaag,
    pub doorbreekt: Bouwlaag,
    pub onderdeel: String,
    pub nlsfb: NlSfb,
    pub materiaal: String,
    /// Sparingsvolume: contour in projectcoordinaten (= trapgat).
    pub sparing: Vec<P2>,
    pub sparing_bron: String,
    pub bron_bestand: String,
    pub bron_sha256: String,
}

const VERBODEN_IN_TRAP: [&str; 3] = ["naam", "project", "niveaus"];

impl ContractInvoer {
    /// Toets de invoer en bouw de trapinvoer voor `bouw::ontwerp`.
    pub fn naar_invoer(&self, project: &ProjectOorsprong) -> Result<Invoer, Fout> {
        if self.schema != INVOER_SCHEMA {
            return Err(Fout::Invoer(format!("schema '{}', verwacht '{INVOER_SCHEMA}'", self.schema)));
        }
        let o = &self.onderdeel;
        let mut c = o.chars();
        if !c.next().is_some_and(|k| k.is_ascii_lowercase() || k.is_ascii_digit())
            || !c.all(|k| k.is_ascii_lowercase() || k.is_ascii_digit() || k == '-')
        {
            return Err(Fout::Invoer(format!("onderdeel '{o}' voldoet niet aan ^[a-z0-9][a-z0-9-]*$")));
        }
        if let Some(n) = &self.nlsfb {
            nlsfb_geldig(&n.code)?;
        }
        project.valideer()?;
        let laag = project.bouwlaag(&self.bouwlaag)?;
        let door = self.doorbreekt_laag(project)?;
        if door.peil <= laag.peil {
            return Err(Fout::Invoer(format!(
                "doorbreekt '{}' (peil {}) ligt niet boven bouwlaag '{}' (peil {})",
                door.code, door.peil, laag.code, laag.peil
            )));
        }
        let vloer = door.vloerpakket.ok_or_else(|| {
            Fout::Invoer(format!("bouwlaag '{}' mist 'vloerpakket' (nodig voor trapgat en sparing)", door.code))
        })?;
        let mut t = self.trap.clone();
        for k in VERBODEN_IN_TRAP {
            if t.contains_key(k) {
                return Err(Fout::Invoer(format!(
                    "trap.{k} hoort niet in de headless invoer: volgt uit het contract"
                )));
            }
        }
        let materiaal = self.houtsoort.naakt();
        for deel in ["boom", "trappaal"] {
            if let Some(serde_json::Value::Object(m)) = t.get_mut(deel) {
                if m.contains_key("materiaal") {
                    return Err(Fout::Invoer(format!(
                        "trap.{deel}.materiaal hoort niet in de headless invoer: gebruik 'houtsoort'"
                    )));
                }
                m.insert("materiaal".into(), materiaal.clone().into());
            }
        }
        if !t.contains_key("boom") {
            t.insert("boom".into(), serde_json::json!({ "materiaal": materiaal }));
        }
        t.insert("naam".into(), self.trapnaam().into());
        t.insert(
            "project".into(),
            serde_json::json!({ "naam": project.project.naam, "bouwlaag": laag.naam, "z_bouwlaag": laag.peil }),
        );
        t.insert(
            "niveaus".into(),
            serde_json::json!({ "onder": laag.peil, "boven": door.peil, "vloerdikte_boven": vloer }),
        );
        Ok(serde_json::from_value(serde_json::Value::Object(t))?)
    }

    pub fn trapnaam(&self) -> String {
        self.naam.clone().unwrap_or_else(|| self.onderdeel.clone())
    }

    pub fn doorbreekt_laag<'a>(&self, project: &'a ProjectOorsprong) -> Result<&'a Bouwlaag, Fout> {
        match &self.doorbreekt {
            Some(c) => project.bouwlaag(c),
            None => project.bouwlaag_erboven(&self.bouwlaag),
        }
    }
}

/// NL-SfB-code voor een trap (`24.1x` of ladder `24.3x`), zoals de IDS eist.
fn nlsfb_geldig(code: &str) -> Result<(), Fout> {
    if matches!(code, "24.10" | "24.11" | "24.12" | "24.13" | "24.15" | "24.30" | "24.31") {
        Ok(())
    } else {
        Err(Fout::Invoer(format!(
            "NL-SfB '{code}' is geen trapcode uit BNA1991 (24.10-24.15, 24.30/24.31)"
        )))
    }
}

/// NL-SfB uit de IFC-trapvorm (lijst BNA1991, nlsfb_bna1991__assembly_code.csv).
pub fn nlsfb_uit_vorm(vorm: &str) -> NlSfb {
    if vorm == "STRAIGHT_RUN_STAIR" {
        NlSfb { code: "24.11".into(), omschrijving: "trappen en hellingen; trappen, rechte steektrappen".into() }
    } else {
        NlSfb {
            code: "24.12".into(),
            omschrijving: "trappen en hellingen; trappen, niet-rechte steektrappen".into(),
        }
    }
}

/// JSON-schema van de headless invoer.
pub fn schema() -> String {
    let s = schemars::schema_for!(ContractInvoer);
    serde_json::to_string_pretty(&s).unwrap_or_default() + "\n"
}

/// SHA-256 als hex (herkomst van de invoer in KBA_Assemblage.BronSha256).
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::Sha256;
    let d = Sha256::digest(data);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gouden vectoren uit `ref/tests/test_contract_guid.py` (Python-referentie).
    #[test]
    fn gouden_vectoren_gelijk_aan_python() {
        let v = [
            ("TEST-0001", "trap-01", "IfcStair", "0SFDsN65bISe_k$pWngdAB"),
            ("TEST-0001", "_project", "IfcSite", "1GTcDAeNfIgRaWIqTGi90h"),
            ("TEST-0001", "_project", "IfcBuildingStorey:00", "2lDP_OR2XUCPKlhRig6pwi"),
            ("TEST-0001", "trap-01", "IfcBuildingElementPart:trede 01", "3KH5pO4orPpg_X3PNaTWIo"),
        ];
        for (p, o, e, g) in v {
            assert_eq!(contract_guid(p, o, e).unwrap(), g, "{p}|{o}|{e}");
        }
    }

    #[test]
    fn normalisatie() {
        assert_eq!(normaliseer("  Trede\t 01 ").unwrap(), "trede 01");
        // NFC: e + combining acute == e-acute
        assert_eq!(normaliseer("tre\u{0301}de").unwrap(), normaliseer("tr\u{e9}de").unwrap());
        assert!(normaliseer("   ").is_err());
        assert!(normaliseer("a|b").is_err());
        assert_eq!(
            contract_guid("TEST-0001", "trap-01", "IfcStair").unwrap(),
            contract_guid(" test-0001 ", "TRAP-01", "ifcstair").unwrap()
        );
    }

    #[test]
    fn laagcodes() {
        for c in ["00", "-1a", "01", "-02", "00a"] {
            assert_eq!(laagcode_geldig(c), c != "-1a", "{c}");
        }
        assert!(!laagcode_geldig("1"));
        assert!(!laagcode_geldig("001"));
    }

    #[test]
    fn houtsoort_naakt() {
        assert_eq!(Houtsoort::Vuren.naakt(), "hout_vuren_generiek");
        assert_eq!(Houtsoort::Mdf.naakt(), "hout_mdf_generiek");
    }
}
