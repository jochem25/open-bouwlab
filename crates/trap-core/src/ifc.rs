//! Trapmodel naar IFC4 (STEP P21), op het niveau van de Python-referentie v5.
//!
//! Entiteitkeuze en indeling volgen `trap_ifc.py` (ai-tekenaar, O-2026-10-29-06):
//! - IfcStair QUARTER_WINDING_STAIR, met IfcStairFlight WINDER (treden en
//!   stootborden als IfcBuildingElementPart USERDEFINED TREAD/LANDING_NOSING/RISER)
//!   en IfcMember STRINGER (boomdelen) en POST (trappaal);
//! - geometrie als IfcExtrudedAreaSolid van een IfcIndexedPolyCurve met
//!   IfcLineIndex en IfcArcIndex (echte bogen), nesten via IfcBooleanResult;
//!   sinds fase 2 geen verstek meer (koude hoekverbinding, besluit Jochem
//!   30-10-2026), dus ook geen IfcHalfSpaceSolid;
//! - clearance als representatie 'Clearance' (Brep) op de IfcStair zelf (IFC4
//!   ADD1 concept template Clearance Geometry), rood transparant.
//!
//! Eenheden: mm. Elementen staan in de bouwlaag; geometrie is t.o.v. de bouwlaag
//! (z - z_bouwlaag).

//!
//! Met `IfcOpties::contract` (assemblagecontract kba-assemblage/1, O-2026-11-15-05)
//! komen daar bij: GlobalIds volgens de contractregel, ruimtelijke structuur en
//! IfcMapConversion uit project.json, NL-SfB, NAA.K.T.-materiaal op elk deel,
//! pset KBA_Assemblage en een sparingsvolume (= trapgat). Zonder contract is de
//! uitvoer byte-gelijk aan die van fe8e6b6.

use std::collections::HashMap;

use crate::contract::{contract_guid, ContractIfc, CONTRACT_VERSIE, PROJECT_ONDERDEEL};
use crate::model::{
    boog_midden, Boomdeel, Clearance, Prisma, Profielelement, Trapmodel, Waarde, P2,
};
use crate::p21::{boolean, guid, lijst, real, reals, string, Bestand, Ref};
use crate::Fout;

/// Klassenaam NL-SfB in IfcClassification (contract K3).
pub const NLSFB_NAAM: &str = "NL-SfB tabel 1";
pub const NLSFB_EDITIE: &str = "BNA1991";
pub const NLSFB_BRON: &str = "BNA1991 (Revit UniformatClassifications, KBA-lijst)";
pub const SPARING_TYPE: &str = "SPARINGSVOLUME";

pub const CLEARANCE_KLEUR: [f64; 3] = [1.0, 0.0, 0.0];
pub const CLEARANCE_TRANSPARANTIE: f64 = 0.7;

#[derive(Debug, Clone)]
pub struct IfcOpties {
    /// FILE_NAME-tijdstempel (ISO 8601); de kern leest geen klok.
    pub tijd: String,
    pub bestandsnaam: String,
    /// Pset KBA_Status op de IfcStair (status van de uitvoer).
    pub disclaimer: Option<String>,
    /// Hoe de clearance in het bestand komt.
    pub clearance: ClearanceIfc,
    /// Assemblagecontract; `None` = uitvoer zoals voor het contract.
    pub contract: Option<ContractIfc>,
}

/// Uitvoervorm van de clearance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ClearanceIfc {
    /// Representatie 'Clearance' (Brep) op de IfcStair (BIMcollab Zoom toont die).
    #[default]
    Representatie,
    /// Idem, plus een IfcBuildingElementProxy 'Clearance' met dezelfde schil als
    /// 'Body' (Revit leest bij een IFC-koppeling alleen 'Body'); type CLEARANCE.
    RepresentatieEnProxy,
    /// Geen clearance-geometrie (IFC voor de leverancier); de pset blijft.
    Geen,
}

impl Default for IfcOpties {
    fn default() -> Self {
        Self {
            tijd: "1970-01-01T00:00:00".into(),
            bestandsnaam: String::new(),
            disclaimer: None,
            clearance: ClearanceIfc::Representatie,
            contract: None,
        }
    }
}

/// GUID-administratie in contractmodus: sleutel per GUID (botsingstoets).
struct Gids {
    project: String,
    onderdeel: String,
    gezien: HashMap<String, String>,
    fouten: Vec<String>,
}

/// Contractsleutel van een object: ruimtelijk (`_project`) of van het onderdeel.
enum Sl<'a> {
    Ruimte(&'a str),
    Deel(&'a str),
}

const SYSTEEM: &str = "open-bouwlab trap-core";

/// Meettype per eigenschap van Pset_StairCommon / Pset_StairFlightCommon
/// (IFC4 propertyset-templates). Onbekend: afgeleid van de waarde.
fn meettype(naam: &str, w: &Waarde) -> &'static str {
    match naam {
        "NumberOfRiser" | "NumberOfTreads" => "IFCCOUNTMEASURE",
        "RiserHeight"
        | "TreadLength"
        | "WalkingLineOffset"
        | "TreadLengthAtOffset"
        | "TreadLengthAtInnerSide"
        | "WaistThickness" => "IFCPOSITIVELENGTHMEASURE",
        "NosingLength" => "IFCLENGTHMEASURE",
        _ => standaard_type(w),
    }
}

fn standaard_type(w: &Waarde) -> &'static str {
    match w {
        Waarde::Bool(_) => "IFCBOOLEAN",
        Waarde::Int(_) => "IFCINTEGER",
        Waarde::Real(_) => "IFCREAL",
        Waarde::Label(_) => "IFCLABEL",
    }
}

fn waarde_tekst(ty: &str, w: &Waarde) -> String {
    let inhoud = match (ty, w) {
        (_, Waarde::Bool(b)) => boolean(*b),
        ("IFCINTEGER", Waarde::Int(i)) => i.to_string(),
        (_, Waarde::Int(i)) => real(*i as f64),
        (_, Waarde::Real(x)) => real(*x),
        (_, Waarde::Label(s)) => string(s),
    };
    format!("{ty}({inhoud})")
}

const STEEK_EIGENSCHAPPEN: [&str; 8] = [
    "NumberOfRiser",
    "NumberOfTreads",
    "RiserHeight",
    "TreadLength",
    "NosingLength",
    "WalkingLineOffset",
    "TreadLengthAtOffset",
    "TreadLengthAtInnerSide",
];

struct Schrijver {
    f: Bestand,
    body: Ref,
    model_ctx: Ref,
    laag_plaats: Ref,
    dz: f64,
    gids: Option<Gids>,
}

impl Schrijver {
    /// GlobalId: zonder contract FNV over de oude sleutel (ongewijzigd), met
    /// contract volgens de contractregel (contract.md G).
    fn gid(&mut self, oud: &str, sl: Sl) -> String {
        let Some(g) = self.gids.as_mut() else {
            return guid(oud);
        };
        let (onderdeel, element) = match sl {
            Sl::Ruimte(e) => (PROJECT_ONDERDEEL, e),
            Sl::Deel(e) => (g.onderdeel.as_str(), e),
        };
        match contract_guid(&g.project, onderdeel, element) {
            Ok(id) => {
                let k = format!("{onderdeel}|{element}");
                // elk object hoort een eigen sleutel te hebben: elke herhaling is fout
                if let Some(eerder) = g.gezien.insert(id.clone(), k.clone()) {
                    g.fouten.push(format!("GUID-botsing: '{k}' en '{eerder}'"));
                }
                id
            }
            Err(e) => {
                g.fouten.push(format!("{element}: {e}"));
                guid(oud)
            }
        }
    }

    fn contract(&self) -> bool {
        self.gids.is_some()
    }

    fn punt3(&mut self, p: [f64; 3]) -> Ref {
        self.f.add("IfcCartesianPoint", reals(&p))
    }

    fn richting(&mut self, d: [f64; 3]) -> Ref {
        self.f.add("IfcDirection", reals(&d))
    }

    fn as_volledig(&mut self, o: [f64; 3]) -> Ref {
        let p = self.punt3(o);
        let z = self.richting([0.0, 0.0, 1.0]);
        let x = self.richting([1.0, 0.0, 0.0]);
        self.f.add("IfcAxis2Placement3D", format!("{p},{z},{x}"))
    }

    fn plaatsing(&mut self, rel: Option<Ref>, o: [f64; 3]) -> Ref {
        let a = self.as_volledig(o);
        let rel = rel.map_or("$".to_string(), |r| r.to_string());
        self.f.add("IfcLocalPlacement", format!("{rel},{a}"))
    }

    fn element_plaatsing(&mut self) -> Ref {
        self.plaatsing(Some(self.laag_plaats), [0.0, 0.0, 0.0])
    }

    /// Verticaal prisma als IfcExtrudedAreaSolid; veelhoek tegen de klok in.
    fn prisma(&mut self, p: &Prisma) -> Ref {
        let mut pts: Vec<P2> = p.veelhoek.clone();
        if oppervlak(&pts) < 0.0 {
            pts.reverse();
        }
        let n = pts.len();
        let lijst2 = self.f.add(
            "IfcCartesianPointList2D",
            lijst(pts.iter().map(|q| reals(q))),
        );
        let segs = lijst((0..n).map(|i| format!("IFCLINEINDEX(({},{}))", i + 1, (i + 1) % n + 1)));
        let kromme = self
            .f
            .add("IfcIndexedPolyCurve", format!("{lijst2},{segs},.F."));
        let profiel = self
            .f
            .add("IfcArbitraryClosedProfileDef", format!(".AREA.,$,{kromme}"));
        let o = self.punt3([0.0, 0.0, p.z0 - self.dz]);
        let plaats = self.f.add("IfcAxis2Placement3D", format!("{o},$,$"));
        let r = self.richting([0.0, 0.0, 1.0]);
        self.f.add(
            "IfcExtrudedAreaSolid",
            format!("{profiel},{plaats},{r},{}", real(p.z1 - p.z0)),
        )
    }

    /// Uitslag van een boomdeel over de dikte: lijnen en echte bogen.
    fn profiel_solid(&mut self, deel: &Boomdeel) -> Result<Ref, Fout> {
        let el = &deel.profiel;
        if el.is_empty() {
            return Err(Fout::Model(format!("{}: leeg profiel", deel.naam)));
        }
        let mut coords: Vec<P2> = Vec::new();
        let index = |p: P2, coords: &mut Vec<P2>| {
            coords.push([p[0], p[1] - self.dz]);
            coords.len()
        };
        let eerste = index(el[0].p0(), &mut coords);
        let mut vorige = eerste;
        let mut segs = Vec::with_capacity(el.len());
        for (k, e) in el.iter().enumerate() {
            let laatste = k + 1 == el.len();
            match e {
                Profielelement::Boog {
                    p0,
                    p1,
                    centrum,
                    r,
                    boven,
                } => {
                    let m = index(boog_midden(*p0, *p1, *centrum, *r, *boven), &mut coords);
                    let eind = if laatste {
                        eerste
                    } else {
                        index(*p1, &mut coords)
                    };
                    segs.push(format!("IFCARCINDEX(({vorige},{m},{eind}))"));
                    vorige = eind;
                }
                Profielelement::Lijn { p1, .. } => {
                    let eind = if laatste {
                        eerste
                    } else {
                        index(*p1, &mut coords)
                    };
                    segs.push(format!("IFCLINEINDEX(({vorige},{eind}))"));
                    vorige = eind;
                }
            }
        }
        let lijst2 = self.f.add(
            "IfcCartesianPointList2D",
            lijst(coords.iter().map(|q| reals(q))),
        );
        let kromme = self.f.add(
            "IfcIndexedPolyCurve",
            format!("{lijst2},{},.F.", lijst(segs)),
        );
        let profiel = self.f.add(
            "IfcArbitraryClosedProfileDef",
            format!(".AREA.,{},{kromme}", string(&deel.naam)),
        );
        let (d, n) = (deel.richting, deel.naar_buiten);
        // lokaal X = langs het deel, lokaal Y = omhoog, lokaal Z = extrusie
        let omhoog = n[0] * d[1] - n[1] * d[0];
        let (oorsprong, as_z) = if omhoog > 0.0 {
            (deel.oorsprong, n)
        } else {
            (
                [
                    deel.oorsprong[0] + deel.dikte * n[0],
                    deel.oorsprong[1] + deel.dikte * n[1],
                ],
                [-n[0], -n[1]],
            )
        };
        let o = self.punt3([oorsprong[0], oorsprong[1], 0.0]);
        let z = self.richting([as_z[0], as_z[1], 0.0]);
        let x = self.richting([d[0], d[1], 0.0]);
        let plaats = self.f.add("IfcAxis2Placement3D", format!("{o},{z},{x}"));
        let r = self.richting([0.0, 0.0, 1.0]);
        Ok(self.f.add(
            "IfcExtrudedAreaSolid",
            format!("{profiel},{plaats},{r},{}", real(deel.dikte)),
        ))
    }

    fn boomdeel(
        &mut self,
        deel: &Boomdeel,
        nesten: &[Prisma],
    ) -> Result<(Ref, &'static str), Fout> {
        let mut item = self.profiel_solid(deel)?;
        let mut soort = "SweptSolid";
        for p in nesten {
            let n = self.prisma(p);
            item = self
                .f
                .add("IfcBooleanResult", format!(".DIFFERENCE.,{item},{n}"));
            soort = "CSG";
        }
        Ok((item, soort))
    }

    fn vorm(&mut self, ctx: Ref, id: &str, soort: &str, items: &[Ref]) -> Ref {
        let rep = self.f.add(
            "IfcShapeRepresentation",
            format!("{ctx},{},{},{}", string(id), string(soort), lijst(items)),
        );
        self.f
            .add("IfcProductDefinitionShape", format!("$,$,({rep})"))
    }

    /// Pset met de contractsleutel `IfcPropertySet:<naam>@<sleutels dragers>`.
    fn pset(
        &mut self,
        el: &[Ref],
        naam: &str,
        waarden: &[(String, Waarde)],
        sleutel: &str,
        dragers: &[&str],
    ) {
        let props: Vec<Ref> = waarden
            .iter()
            .map(|(k, w)| {
                let ty = if naam.starts_with("Pset_") {
                    meettype(k, w)
                } else {
                    standaard_type(w)
                };
                self.f.add(
                    "IfcPropertySingleValue",
                    format!("{},$,{},$", string(k), waarde_tekst(ty, w)),
                )
            })
            .collect();
        self.pset_props(el, naam, props, sleutel, dragers);
    }

    /// Pset met expliciete IFC-typen: (eigenschap, type, opgemaakte waarde).
    fn pset_getypt(&mut self, el: &[Ref], naam: &str, waarden: &[(&str, &str, String)], dragers: &[&str]) {
        let props: Vec<Ref> = waarden
            .iter()
            .map(|(k, ty, w)| {
                self.f.add("IfcPropertySingleValue", format!("{},$,{ty}({w}),$", string(k)))
            })
            .collect();
        self.pset_props(el, naam, props, "", dragers);
    }

    fn pset_props(&mut self, el: &[Ref], naam: &str, props: Vec<Ref>, sleutel: &str, dragers: &[&str]) {
        let mut d: Vec<&str> = dragers.to_vec();
        d.sort_unstable();
        let ck = format!("{naam}@{}", d.join(","));
        let g_ps = self.gid(&format!("pset|{sleutel}|{naam}"), Sl::Deel(&format!("IfcPropertySet:{ck}")));
        let ps = self.f.add(
            "IfcPropertySet",
            format!("'{g_ps}',$,{},$,{}", string(naam), lijst(props)),
        );
        let g_rel =
            self.gid(&format!("rdp|{sleutel}|{naam}"), Sl::Deel(&format!("IfcRelDefinesByProperties:{ck}")));
        self.f.add(
            "IfcRelDefinesByProperties",
            format!("'{g_rel}',$,$,$,{},{ps}", lijst(el)),
        );
    }

    /// IfcRelAggregates; contractsleutel `IfcRelAggregates:<sleutel geheel>`.
    fn rel_aggregates(&mut self, sleutel: &str, geheel: Ref, delen: &[Ref], sl: Sl) {
        let g = self.gid(&format!("agg|{sleutel}"), sl);
        self.f.add(
            "IfcRelAggregates",
            format!("'{g}',$,$,$,{geheel},{}", lijst(delen)),
        );
    }

    /// Clearance-schil als representatie `id` ('Clearance' of 'Body') in `ctx`.
    fn clearance(&mut self, c: &Clearance, ctx: Ref, id: &str) -> Result<Ref, Fout> {
        let punten: Vec<Ref> = c
            .hoekpunten
            .iter()
            .map(|p| self.punt3([p[0], p[1], p[2] - self.dz]))
            .collect();
        let mut vlakken = Vec::with_capacity(c.vlakken.len());
        for v in &c.vlakken {
            if v.len() < 3 || v.iter().any(|&i| i >= punten.len()) {
                return Err(Fout::Model(format!("clearance: ongeldig vlak {v:?}")));
            }
            let lus = self
                .f
                .add("IfcPolyLoop", lijst(v.iter().map(|&i| punten[i])));
            let rand = self.f.add("IfcFaceOuterBound", format!("{lus},.T."));
            vlakken.push(self.f.add("IfcFace", format!("({rand})")));
        }
        let schil = self.f.add("IfcClosedShell", lijst(vlakken));
        let brep = self.f.add("IfcFacetedBrep", schil.to_string());
        let [r, g, b] = CLEARANCE_KLEUR;
        let kleur = self.f.add(
            "IfcColourRgb",
            format!("$,{},{},{}", real(r), real(g), real(b)),
        );
        let weergave = self.f.add(
            "IfcSurfaceStyleRendering",
            format!(
                "{kleur},{},$,$,$,$,$,$,.NOTDEFINED.",
                real(CLEARANCE_TRANSPARANTIE)
            ),
        );
        let stijl = self.f.add(
            "IfcSurfaceStyle",
            format!(
                "{},.BOTH.,({weergave})",
                string("clearance rood transparant")
            ),
        );
        self.f.add("IfcStyledItem", format!("{brep},({stijl}),$"));
        Ok(self.f.add(
            "IfcShapeRepresentation",
            format!("{ctx},{},'Brep',({brep})", string(id)),
        ))
    }
}

/// Signed area (positief = tegen de klok in).
fn oppervlak(p: &[P2]) -> f64 {
    let n = p.len();
    (0..n)
        .map(|i| {
            let (a, b) = (p[i], p[(i + 1) % n]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

fn soort_onderdeel(s: &str) -> Result<&'static str, Fout> {
    match s {
        "TREDE" => Ok("TREAD"),
        "WELSTUK" => Ok("LANDING_NOSING"),
        "STOOTBORD" => Ok("RISER"),
        _ => Err(Fout::Model(format!("onbekende soort onderdeel '{s}'"))),
    }
}

/// Schrijf het trapmodel als IFC4 STEP-tekst.
pub fn schrijf_ifc(m: &Trapmodel, opties: &IfcOpties) -> Result<String, Fout> {
    let mut f = Bestand::new();
    let sl = &m.trap.naam; // sleutel voor deterministische GUID's (zonder contract)
    let c = opties.contract.as_ref();
    let z_laag = c.map_or(m.z_bouwlaag, |c| c.laag.peil);

    // project, eenheden, contexten
    let project = f.add("IfcProject", "");
    let lengte = f.add("IfcSIUnit", "*,.LENGTHUNIT.,.MILLI.,.METRE.");
    let hoek = f.add("IfcSIUnit", "*,.PLANEANGLEUNIT.,$,.RADIAN.");
    let eenheden = f.add("IfcUnitAssignment", format!("({lengte},{hoek})"));
    let mut s = Schrijver {
        f,
        body: Ref(0),
        model_ctx: Ref(0),
        laag_plaats: Ref(0),
        dz: z_laag,
        gids: c.map(|c| Gids {
            project: c.project.project.id.clone(),
            onderdeel: c.onderdeel.clone(),
            gezien: HashMap::new(),
            fouten: Vec::new(),
        }),
    };
    let wca = s.as_volledig([0.0, 0.0, 0.0]);
    s.model_ctx = s.f.add(
        "IfcGeometricRepresentationContext",
        format!("$,'Model',3,1.E-05,{wca},$"),
    );
    s.body = s.f.add(
        "IfcGeometricRepresentationSubContext",
        format!("'Body','Model',*,*,*,*,{},$,.MODEL_VIEW.,$", s.model_ctx),
    );
    // IfcProject staat vooraan (#1); de argumenten volgen aan het eind, als de
    // context bekend is.
    if let Some(c) = c {
        georef(&mut s, c);
    }

    // ruimtelijke structuur
    let (site_naam, geb_naam, laag_naam, laag_code) = match c {
        Some(c) => (
            c.project.project.site.clone(),
            c.project.project.gebouw.clone(),
            c.laag.naam.clone(),
            c.laag.code.clone(),
        ),
        None => ("site".into(), "gebouw".into(), m.bouwlaag.clone(), String::new()),
    };
    let laag_sl = format!("IfcBuildingStorey:{laag_code}");
    let site_pl = s.plaatsing(None, [0.0, 0.0, 0.0]);
    let g = s.gid(&format!("site|{sl}"), Sl::Ruimte("IfcSite"));
    let site = s.f.add(
        "IfcSite",
        format!("'{g}',$,{},$,$,{site_pl},$,$,$,$,$,$,$,$", string(&site_naam)),
    );
    let geb_pl = s.plaatsing(Some(site_pl), [0.0, 0.0, 0.0]);
    let g = s.gid(&format!("gebouw|{sl}"), Sl::Ruimte("IfcBuilding"));
    let gebouw = s.f.add(
        "IfcBuilding",
        format!("'{g}',$,{},$,$,{geb_pl},$,$,$,$,$,$", string(&geb_naam)),
    );
    s.laag_plaats = s.plaatsing(Some(geb_pl), [0.0, 0.0, z_laag]);
    let g = s.gid(&format!("laag|{sl}"), Sl::Ruimte(&laag_sl));
    let laag = s.f.add(
        "IfcBuildingStorey",
        format!(
            "'{g}',$,{},$,$,{},$,$,$,{}",
            string(&laag_naam),
            s.laag_plaats,
            real(z_laag)
        ),
    );
    s.rel_aggregates(&format!("project|{sl}"), project, &[site], Sl::Ruimte("IfcRelAggregates:IfcProject"));
    s.rel_aggregates(&format!("site|{sl}"), site, &[gebouw], Sl::Ruimte("IfcRelAggregates:IfcSite"));
    s.rel_aggregates(&format!("gebouw|{sl}"), gebouw, &[laag], Sl::Ruimte("IfcRelAggregates:IfcBuilding"));

    // treden, welstuk, stootborden
    let mut delen = Vec::new();
    let mut sleutels: Vec<String> = Vec::new(); // contractsleutels van alle delen (materiaal)
    for p in m.treden.iter().chain(&m.borden) {
        let soort = soort_onderdeel(&p.soort)?;
        let item = s.prisma(p);
        let vorm = s.vorm(s.body, "Body", "SweptSolid", &[item]);
        let pl = s.element_plaatsing();
        let k = format!("IfcBuildingElementPart:{}", p.naam);
        let g = s.gid(&format!("deel|{sl}|{}", p.naam), Sl::Deel(&k));
        sleutels.push(k);
        delen.push(s.f.add(
            "IfcBuildingElementPart",
            format!(
                "'{g}',$,{},$,{},{pl},{vorm},$,.USERDEFINED.",
                string(&p.naam),
                string(soort)
            ),
        ));
    }

    // bomen en palen
    let mut leden = Vec::new();
    for boom in &m.bomen {
        for bd in &boom.delen {
            let (item, soort) = s.boomdeel(&bd.deel, &bd.nesten)?;
            let vorm = s.vorm(s.body, "Body", soort, &[item]);
            let pl = s.element_plaatsing();
            let k = format!("IfcMember:{}", bd.deel.naam);
            let g = s.gid(&format!("lid|{sl}|{}", bd.deel.naam), Sl::Deel(&k));
            sleutels.push(k);
            leden.push(s.f.add(
                "IfcMember",
                format!(
                    "'{g}',$,{},$,$,{pl},{vorm},$,.STRINGER.",
                    string(&bd.deel.naam)
                ),
            ));
        }
    }
    for paal in &m.palen {
        let mut item = s.prisma(&paal.paal);
        let mut soort = "SweptSolid";
        for p in &paal.nesten {
            let n = s.prisma(p);
            item =
                s.f.add("IfcBooleanResult", format!(".DIFFERENCE.,{item},{n}"));
            soort = "CSG";
        }
        let vorm = s.vorm(s.body, "Body", soort, &[item]);
        let pl = s.element_plaatsing();
        let k = format!("IfcMember:{}", paal.paal.naam);
        let g = s.gid(&format!("lid|{sl}|{}", paal.paal.naam), Sl::Deel(&k));
        let lid = s.f.add(
            "IfcMember",
            format!(
                "'{g}',$,{},$,$,{pl},{vorm},$,.POST.",
                string(&paal.paal.naam)
            ),
        );
        s.pset(&[lid], "KBA_Trappaal", &paal.eigenschappen, &paal.paal.naam, &[&k]);
        // met contract: een materiaal voor de hele trap (hieronder)
        if let (false, Some((_, Waarde::Label(mat)))) = (
            s.contract(),
            paal.eigenschappen.iter().find(|(k, _)| k == "Materiaal"),
        ) {
            let materiaal = s.f.add("IfcMaterial", format!("{},$,'wood'", string(mat)));
            s.f.add(
                "IfcRelAssociatesMaterial",
                format!(
                    "'{}',$,$,$,({lid}),{materiaal}",
                    guid(&format!("mat|{sl}|{}", paal.paal.naam))
                ),
            );
        }
        sleutels.push(k);
        leden.push(lid);
    }

    // trap en steek
    let steek_pl = s.element_plaatsing();
    let t = &m.trap;
    // contract: staptitel zonder trapnaam (stabiele sleutel)
    let steek_naam = if s.contract() { "steek".to_string() } else { format!("{} - steek", t.naam) };
    let steek_k = format!("IfcStairFlight:{steek_naam}");
    let g = s.gid(&format!("steek|{sl}"), Sl::Deel(&steek_k));
    let steek = s.f.add(
        "IfcStairFlight",
        format!(
            "'{g}',$,{},$,$,{steek_pl},$,$,{},{},{},{},.{}.",
            string(&steek_naam),
            t.optreden,
            t.aantreden,
            real(t.optrede),
            real(t.aantrede),
            t.steek
        ),
    );
    let trap_pl = s.element_plaatsing();
    let clearance = m.clearance.as_ref().filter(|_| opties.clearance != ClearanceIfc::Geen);
    let trap_vorm = match clearance {
        Some(c) => {
            let ctx = s.f.add(
                "IfcGeometricRepresentationSubContext",
                format!(
                    "'Clearance','Model',*,*,*,*,{},$,.MODEL_VIEW.,$",
                    s.model_ctx
                ),
            );
            let rep = s.clearance(c, ctx, "Clearance")?;
            s.f.add("IfcProductDefinitionShape", format!("$,$,({rep})"))
                .to_string()
        }
        None => "$".to_string(),
    };
    let trap_k = "IfcStair";
    let g = s.gid(&format!("trap|{sl}"), Sl::Deel(trap_k));
    let trap = s.f.add(
        "IfcStair",
        format!(
            "'{g}',$,{},$,$,{trap_pl},{trap_vorm},$,.{}.",
            string(&t.naam),
            t.vorm
        ),
    );
    s.rel_aggregates(&format!("steek|{sl}"), steek, &delen, Sl::Deel(&format!("IfcRelAggregates:{steek_k}")));
    let mut trapdelen = vec![steek];
    trapdelen.extend(&leden);
    s.rel_aggregates(&format!("trap|{sl}"), trap, &trapdelen, Sl::Deel("IfcRelAggregates:IfcStair"));
    let mut in_laag = vec![trap];
    if let (Some(cl), ClearanceIfc::RepresentatieEnProxy) = (clearance, opties.clearance) {
        let rep = s.clearance(cl, s.body, "Body")?;
        let vorm = s.f.add("IfcProductDefinitionShape", format!("$,$,({rep})"));
        let pl = s.element_plaatsing();
        let proxy_k = "IfcBuildingElementProxy:Clearance";
        let type_k = "IfcBuildingElementProxyType:CLEARANCE";
        let g = s.gid(&format!("clearance-proxy|{sl}"), Sl::Deel(proxy_k));
        let proxy = s.f.add(
            "IfcBuildingElementProxy",
            format!("'{g}',$,'Clearance',$,'CLEARANCE',{pl},{vorm},$,.USERDEFINED."),
        );
        let g = s.gid(&format!("clearance-type|{sl}"), Sl::Deel(type_k));
        let ptype = s.f.add(
            "IfcBuildingElementProxyType",
            format!("'{g}',$,'CLEARANCE',$,$,$,$,$,'CLEARANCE',.USERDEFINED."),
        );
        let g = s.gid(
            &format!("clearance-reltype|{sl}"),
            Sl::Deel(&format!("IfcRelDefinesByType:{type_k}")),
        );
        s.f.add(
            "IfcRelDefinesByType",
            format!("'{g}',$,$,$,({proxy}),{ptype}"),
        );
        s.pset(&[proxy], "KBA_Clearance", &cl.eigenschappen, &format!("clearance-proxy|{sl}"), &[proxy_k]);
        in_laag.push(proxy);
    }
    if let Some(c) = c {
        in_laag.push(sparing(&mut s, c)?);
    }
    let g = s.gid(
        &format!("bevat|{sl}"),
        Sl::Deel(&format!("IfcRelContainedInSpatialStructure:{laag_code}")),
    );
    s.f.add(
        "IfcRelContainedInSpatialStructure",
        format!("'{g}',$,$,$,{},{laag}", lijst(in_laag)),
    );
    s.pset(&[trap], "Pset_StairCommon", &m.pset_trap, sl, &[trap_k]);
    let steek_props: Vec<(String, Waarde)> = m
        .pset_trap
        .iter()
        .filter(|(k, _)| STEEK_EIGENSCHAPPEN.contains(&k.as_str()))
        .cloned()
        .collect();
    s.pset(&[steek], "Pset_StairFlightCommon", &steek_props, sl, &[&steek_k]);
    s.pset(&[trap], "KBA_Trapverdrijving", &m.pset_extra, sl, &[trap_k]);
    if let Some(cl) = &m.clearance {
        s.pset(&[trap], "KBA_Clearance", &cl.eigenschappen, sl, &[trap_k]);
    }
    if let Some(d) = &opties.disclaimer {
        s.pset(
            &[trap],
            "KBA_Status",
            &[("Disclaimer".to_string(), Waarde::Label(d.clone()))],
            sl,
            &[trap_k],
        );
    }

    // contract K: classificatie, materiaal, herkomst
    if let Some(c) = c {
        let klas = s.f.add(
            "IfcClassification",
            format!("{},{},$,{},$,$,$", string(NLSFB_BRON), string(NLSFB_EDITIE), string(NLSFB_NAAM)),
        );
        let refc = s.f.add(
            "IfcClassificationReference",
            format!("$,{},{},{klas},$,$", string(&c.nlsfb.code), string(&c.nlsfb.omschrijving)),
        );
        let g = s.gid("", Sl::Deel(&format!("IfcRelAssociatesClassification:{trap_k}")));
        s.f.add(
            "IfcRelAssociatesClassification",
            format!("'{g}',$,$,$,({trap}),{refc}"),
        );
        let mat = s.f.add("IfcMaterial", format!("{},$,'wood'", string(&c.materiaal)));
        let mut alle: Vec<Ref> = vec![trap, steek];
        alle.extend(&delen);
        alle.extend(&leden);
        let mut ks: Vec<&str> = vec![trap_k, &steek_k];
        ks.extend(sleutels.iter().map(String::as_str));
        ks.sort_unstable();
        let g = s.gid("", Sl::Deel(&format!("IfcRelAssociatesMaterial:{}", ks.join(","))));
        s.f.add(
            "IfcRelAssociatesMaterial",
            format!("'{g}',$,$,$,{},{mat}", lijst(alle)),
        );
        s.pset_getypt(
            &[trap],
            "KBA_Assemblage",
            &[
                ("Contractversie", "IFCLABEL", string(CONTRACT_VERSIE)),
                ("Project", "IFCIDENTIFIER", string(&c.project.project.id)),
                ("Onderdeel", "IFCIDENTIFIER", string(&c.onderdeel)),
                ("Bouwlaag", "IFCIDENTIFIER", string(&c.laag.code)),
                ("BronBestand", "IFCLABEL", string(&c.bron_bestand)),
                ("BronSha256", "IFCLABEL", string(&c.bron_sha256)),
            ],
            &[trap_k],
        );
    }

    let gp = s.gid(&format!("project|{sl}"), Sl::Ruimte("IfcProject"));
    let projectnaam = c.map_or(m.projectnaam.clone(), |c| c.project.project.naam.clone());
    if let Some(g) = &s.gids {
        if !g.fouten.is_empty() {
            return Err(Fout::Model(format!("contract: {}", g.fouten.join("; "))));
        }
    }
    let mut f = s.f;
    f.vervang(
        project,
        "IfcProject",
        format!(
            "'{gp}',$,{},$,$,$,$,({}),{eenheden}",
            string(&projectnaam),
            s.model_ctx
        ),
    );
    Ok(f.tekst(&opties.bestandsnaam, &opties.tijd, SYSTEEM))
}

/// IfcMapConversion van de modelcontext naar RD (EPSG:28992) en NAP (contract O4).
fn georef(s: &mut Schrijver, c: &ContractIfc) {
    let o = &c.project.oorsprong;
    let meter = s.f.add("IfcSIUnit", "*,.LENGTHUNIT.,$,.METRE.");
    let crs = s.f.add(
        "IfcProjectedCRS",
        format!("'EPSG:28992','Amersfoort / RD New',$,'NAP',$,$,{meter}"),
    );
    let a = o.rotatie_ware_noorden.to_radians();
    s.f.add(
        "IfcMapConversion",
        format!(
            "{},{crs},{},{},{},{},{},{}",
            s.model_ctx,
            real(o.rd_x / 1000.0),
            real(o.rd_y / 1000.0),
            real(o.nap_peil / 1000.0),
            real(a.cos()),
            real(a.sin()),
            real(0.001)
        ),
    );
}

/// Sparingsvolume (contract S): proxy SPARINGSVOLUME met de trapgatcontour, van
/// o.k. vloerpakket tot peil van de doorbroken bouwlaag.
fn sparing(s: &mut Schrijver, c: &ContractIfc) -> Result<Ref, Fout> {
    let door = &c.doorbreekt;
    let vloer = door
        .vloerpakket
        .ok_or_else(|| Fout::Invoer(format!("bouwlaag '{}' mist 'vloerpakket'", door.code)))?;
    if c.sparing.len() < 3 {
        return Err(Fout::Model("sparing: trapgat heeft minder dan 3 punten".into()));
    }
    let prisma = Prisma {
        naam: "sparing".into(),
        soort: "SPARING".into(),
        veelhoek: c.sparing.clone(),
        z0: door.peil - vloer,
        z1: door.peil,
    };
    let item = s.prisma(&prisma);
    let vorm = s.vorm(s.body, "Body", "SweptSolid", &[item]);
    let pl = s.element_plaatsing();
    let naam = format!("sparing {} {}", door.code, c.onderdeel);
    let k = format!("IfcBuildingElementProxy:{naam}");
    let g = s.gid("", Sl::Deel(&k));
    let proxy = s.f.add(
        "IfcBuildingElementProxy",
        format!(
            "'{g}',$,{},{},'{SPARING_TYPE}',{pl},{vorm},$,.USERDEFINED.",
            string(&naam),
            string(&format!("sparingsvolume van {} in de vloer van bouwlaag {}", c.onderdeel, door.naam))
        ),
    );
    s.pset_getypt(
        &[proxy],
        "KBA_Sparing",
        &[
            ("Onderdeel", "IFCIDENTIFIER", string(&c.onderdeel)),
            ("Doorbreekt", "IFCLABEL", string(&door.naam)),
            ("DoorbreektCode", "IFCIDENTIFIER", string(&door.code)),
            ("Bron", "IFCLABEL", string(&c.sparing_bron)),
            ("Marge", "IFCLENGTHMEASURE", real(0.0)),
        ],
        &[&k],
    );
    Ok(proxy)
}
