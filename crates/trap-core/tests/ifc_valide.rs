//! O-2026-10-30-14: de IFC's die het scherm aanbiedt (met clearance-proxy en
//! voor de leverancier) hebben 0 schemafouten volgens ifcopenshell (express
//! rules), voor een rechte trap, een kwartdraai en een halfdraai.
//!
//! Draait `tests/golden/valideer.py` met `python -I`; zonder Python met
//! ifcopenshell slaat de schemacontrole over (met melding), de rest niet.

use std::path::PathBuf;
use std::process::Command;

use trap_core::{ontwerp_uitvoer, Invoer};

const VORMEN: [(&str, &str); 3] = [
    (
        "recht",
        r#"{"naam":"recht","niveaus":{"onder":0,"boven":2800},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[1,0],"binnenzijde":"links",
                           "armlengtes":[3300],"breedte":850}}"#,
    ),
    (
        "kwart",
        r#"{"naam":"kwart","niveaus":{"onder":0,"boven":2800},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[0,1],"binnenzijde":"links",
                           "armlengtes":[2400,1600],"breedte":850},
            "trappaal":{}}"#,
    ),
    (
        "half",
        r#"{"naam":"half","niveaus":{"onder":0,"boven":3000},
            "plattegrond":{"soort":"armen","start":[0,0],"richting":[0,1],"binnenzijde":"rechts",
                           "armlengtes":[1500,2200,1500],"breedte":850},
            "optreden":16,"trappaal":{}}"#,
    ),
];

fn python_met_ifcopenshell() -> bool {
    Command::new("python")
        .args(["-I", "-c", "import ifcopenshell.validate"])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn ifc_varianten_valide() {
    let uit = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/trap-core/valide");
    std::fs::create_dir_all(&uit).unwrap();
    let mut args = Vec::new();
    for (naam, json) in VORMEN {
        let inv: Invoer = serde_json::from_str(json).unwrap();
        let u = ontwerp_uitvoer(&inv, "2026-10-30T00:00:00").expect(naam);
        assert_eq!(u.bestandsnaam, naam);
        assert!(u.ifc.contains("IFCBUILDINGELEMENTPROXY("), "{naam}: proxy");
        assert!(u.ifc.contains("'Clearance','Brep'"), "{naam}: representatie");
        assert!(!u.ifc_leverancier.contains("IFCBUILDINGELEMENTPROXY("), "{naam}: leverancier zonder proxy");
        assert!(!u.ifc_leverancier.contains("'Clearance','Brep'"), "{naam}: leverancier zonder clearance");
        assert!(u.ifc_leverancier.contains("'KBA_Status'"), "{naam}: disclaimer");
        assert!(!u.tekening.treden.is_empty() && !u.tekening.klimlijn.is_empty());
        let (a, b) = (uit.join(format!("{naam}.ifc")), uit.join(format!("{naam}_leverancier.ifc")));
        std::fs::write(&a, &u.ifc).unwrap();
        std::fs::write(&b, &u.ifc_leverancier).unwrap();
        args.push(format!("{}:1:1", a.display()));
        args.push(format!("{}:0:0", b.display()));
    }
    if !python_met_ifcopenshell() {
        eprintln!("overgeslagen: schemacontrole vraagt python met ifcopenshell");
        return;
    }
    let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/valideer.py");
    let o = Command::new("python").arg("-I").arg(script).args(&args).output().unwrap();
    let tekst = String::from_utf8_lossy(&o.stdout);
    println!("{tekst}");
    assert!(o.status.success(), "schemafouten:\n{tekst}\n{}", String::from_utf8_lossy(&o.stderr));
}
