//! Norm-tabellen voor de PV-keten.
//!
//! Bevat de transcriptie van NTA 8800 Tabel 17.2 (opvallende zonnestraling per
//! hellingshoek + oriëntatie) die de tilt/azimut-afhankelijkheid van de
//! PV-opbrengst bepaalt — zie [`irradiation`].

// De tabel staat sinds 12-10-2026 in `nta8800-tables` (ook de zonwinst in de
// demand-keten gebruikt hem); hier alleen het oude pad.
pub use nta8800_tables::climate::tilted as irradiation;

pub use irradiation::{isol_w_per_m2, tilt_azimuth_factor};
