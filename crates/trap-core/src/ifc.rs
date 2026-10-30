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

use crate::model::{
    boog_midden, Boomdeel, Clearance, Prisma, Profielelement, Trapmodel, Waarde, P2,
};
use crate::p21::{boolean, guid, lijst, real, reals, string, Bestand, Ref};
use crate::Fout;

pub const CLEARANCE_KLEUR: [f64; 3] = [1.0, 0.0, 0.0];
pub const CLEARANCE_TRANSPARANTIE: f64 = 0.7;

#[derive(Debug, Clone)]
pub struct IfcOpties {
    /// FILE_NAME-tijdstempel (ISO 8601); de kern leest geen klok.
    pub tijd: String,
    pub bestandsnaam: String,
    /// Pset KBA_Status op de IfcStair (status van de uitvoer).
    pub disclaimer: Option<String>,
}

impl Default for IfcOpties {
    fn default() -> Self {
        Self {
            tijd: "1970-01-01T00:00:00".into(),
            bestandsnaam: String::new(),
            disclaimer: None,
        }
    }
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
}

impl Schrijver {
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

    fn pset(&mut self, el: &[Ref], naam: &str, waarden: &[(String, Waarde)], sleutel: &str) {
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
        let ps = self.f.add(
            "IfcPropertySet",
            format!(
                "'{}',$,{},$,{}",
                guid(&format!("pset|{sleutel}|{naam}")),
                string(naam),
                lijst(props)
            ),
        );
        self.f.add(
            "IfcRelDefinesByProperties",
            format!(
                "'{}',$,$,$,{},{ps}",
                guid(&format!("rdp|{sleutel}|{naam}")),
                lijst(el)
            ),
        );
    }

    fn rel_aggregates(&mut self, sleutel: &str, geheel: Ref, delen: &[Ref]) {
        self.f.add(
            "IfcRelAggregates",
            format!(
                "'{}',$,$,$,{geheel},{}",
                guid(&format!("agg|{sleutel}")),
                lijst(delen)
            ),
        );
    }

    fn clearance(&mut self, c: &Clearance, ctx: Ref) -> Result<Ref, Fout> {
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
            format!("{ctx},'Clearance','Brep',({brep})"),
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
    let sl = &m.trap.naam; // sleutel voor deterministische GUID's

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
        dz: m.z_bouwlaag,
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

    // ruimtelijke structuur
    let site_pl = s.plaatsing(None, [0.0, 0.0, 0.0]);
    let site = s.f.add(
        "IfcSite",
        format!(
            "'{}',$,'site',$,$,{site_pl},$,$,$,$,$,$,$,$",
            guid(&format!("site|{sl}"))
        ),
    );
    let geb_pl = s.plaatsing(Some(site_pl), [0.0, 0.0, 0.0]);
    let gebouw = s.f.add(
        "IfcBuilding",
        format!(
            "'{}',$,'gebouw',$,$,{geb_pl},$,$,$,$,$,$",
            guid(&format!("gebouw|{sl}"))
        ),
    );
    s.laag_plaats = s.plaatsing(Some(geb_pl), [0.0, 0.0, m.z_bouwlaag]);
    let laag = s.f.add(
        "IfcBuildingStorey",
        format!(
            "'{}',$,{},$,$,{},$,$,$,{}",
            guid(&format!("laag|{sl}")),
            string(&m.bouwlaag),
            s.laag_plaats,
            real(m.z_bouwlaag)
        ),
    );
    s.rel_aggregates(&format!("project|{sl}"), project, &[site]);
    s.rel_aggregates(&format!("site|{sl}"), site, &[gebouw]);
    s.rel_aggregates(&format!("gebouw|{sl}"), gebouw, &[laag]);

    // treden, welstuk, stootborden
    let mut delen = Vec::new();
    for p in m.treden.iter().chain(&m.borden) {
        let soort = soort_onderdeel(&p.soort)?;
        let item = s.prisma(p);
        let vorm = s.vorm(s.body, "Body", "SweptSolid", &[item]);
        let pl = s.element_plaatsing();
        delen.push(s.f.add(
            "IfcBuildingElementPart",
            format!(
                "'{}',$,{},$,{},{pl},{vorm},$,.USERDEFINED.",
                guid(&format!("deel|{sl}|{}", p.naam)),
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
            leden.push(s.f.add(
                "IfcMember",
                format!(
                    "'{}',$,{},$,$,{pl},{vorm},$,.STRINGER.",
                    guid(&format!("lid|{sl}|{}", bd.deel.naam)),
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
        let lid = s.f.add(
            "IfcMember",
            format!(
                "'{}',$,{},$,$,{pl},{vorm},$,.POST.",
                guid(&format!("lid|{sl}|{}", paal.paal.naam)),
                string(&paal.paal.naam)
            ),
        );
        s.pset(&[lid], "KBA_Trappaal", &paal.eigenschappen, &paal.paal.naam);
        if let Some((_, Waarde::Label(mat))) =
            paal.eigenschappen.iter().find(|(k, _)| k == "Materiaal")
        {
            let materiaal = s.f.add("IfcMaterial", format!("{},$,'wood'", string(mat)));
            s.f.add(
                "IfcRelAssociatesMaterial",
                format!(
                    "'{}',$,$,$,({lid}),{materiaal}",
                    guid(&format!("mat|{sl}|{}", paal.paal.naam))
                ),
            );
        }
        leden.push(lid);
    }

    // trap en steek
    let steek_pl = s.element_plaatsing();
    let t = &m.trap;
    let steek = s.f.add(
        "IfcStairFlight",
        format!(
            "'{}',$,{},$,$,{steek_pl},$,$,{},{},{},{},.{}.",
            guid(&format!("steek|{sl}")),
            string(&format!("{} - steek", t.naam)),
            t.optreden,
            t.aantreden,
            real(t.optrede),
            real(t.aantrede),
            t.steek
        ),
    );
    let trap_pl = s.element_plaatsing();
    let trap_vorm = match &m.clearance {
        Some(c) => {
            let ctx = s.f.add(
                "IfcGeometricRepresentationSubContext",
                format!(
                    "'Clearance','Model',*,*,*,*,{},$,.MODEL_VIEW.,$",
                    s.model_ctx
                ),
            );
            let rep = s.clearance(c, ctx)?;
            s.f.add("IfcProductDefinitionShape", format!("$,$,({rep})"))
                .to_string()
        }
        None => "$".to_string(),
    };
    let trap = s.f.add(
        "IfcStair",
        format!(
            "'{}',$,{},$,$,{trap_pl},{trap_vorm},$,.{}.",
            guid(&format!("trap|{sl}")),
            string(&t.naam),
            t.vorm
        ),
    );
    s.rel_aggregates(&format!("steek|{sl}"), steek, &delen);
    let mut trapdelen = vec![steek];
    trapdelen.extend(&leden);
    s.rel_aggregates(&format!("trap|{sl}"), trap, &trapdelen);
    s.f.add(
        "IfcRelContainedInSpatialStructure",
        format!("'{}',$,$,$,({trap}),{laag}", guid(&format!("bevat|{sl}"))),
    );
    s.pset(&[trap], "Pset_StairCommon", &m.pset_trap, sl);
    let steek_props: Vec<(String, Waarde)> = m
        .pset_trap
        .iter()
        .filter(|(k, _)| STEEK_EIGENSCHAPPEN.contains(&k.as_str()))
        .cloned()
        .collect();
    s.pset(&[steek], "Pset_StairFlightCommon", &steek_props, sl);
    s.pset(&[trap], "KBA_Trapverdrijving", &m.pset_extra, sl);
    if let Some(c) = &m.clearance {
        s.pset(&[trap], "KBA_Clearance", &c.eigenschappen, sl);
    }
    if let Some(d) = &opties.disclaimer {
        s.pset(
            &[trap],
            "KBA_Status",
            &[("Disclaimer".to_string(), Waarde::Label(d.clone()))],
            sl,
        );
    }

    let mut f = s.f;
    f.vervang(
        project,
        "IfcProject",
        format!(
            "'{}',$,{},$,$,$,$,({}),{eenheden}",
            guid(&format!("project|{sl}")),
            string(&m.projectnaam),
            s.model_ctx
        ),
    );
    Ok(f.tekst(&opties.bestandsnaam, &opties.tijd, SYSTEEM))
}
