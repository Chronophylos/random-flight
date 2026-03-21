mod lnmperf;

pub use lnmperf::import_lnmperf;

/// Convert lbs to kg
pub(crate) fn lbs_to_kg(lbs: f64) -> f64 {
    lbs * 0.453592
}

/// Convert US gallons to kg using fuel density
pub(crate) fn gal_to_kg(gal: f64, jet_fuel: bool) -> f64 {
    let density_kg_per_gal = if jet_fuel { 3.0390 } else { 2.7216 };
    gal * density_kg_per_gal
}
