use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;

use super::{gal_to_kg, lbs_to_kg};

/// Result of importing an LNM performance file.
#[derive(Debug)]
pub struct LnmImportResult {
    pub icao_type: String,
    pub toml_content: String,
    pub warnings: Vec<String>,
}

/// Import a Little Navmap `.lnmperf` file and produce TOML output.
pub fn import_lnmperf(path: &Path) -> Result<LnmImportResult, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    let trimmed = contents.trim_start();
    if trimmed.starts_with("<?xml") || trimmed.starts_with('<') {
        parse_xml(&contents)
    } else {
        parse_ini(&contents)
    }
}

struct RawPerf {
    name: Option<String>,
    aircraft_type: Option<String>,
    fuel_as_volume: bool,
    jet_fuel: bool,
    usable_fuel: Option<f64>,
    min_runway_length: Option<f64>,
    runway_type: Option<String>,
    climb_speed: Option<f64>,
    climb_rate: Option<f64>,
    climb_fuel_flow: Option<f64>,
    cruise_speed: Option<f64>,
    cruise_fuel_flow: Option<f64>,
    descent_speed: Option<f64>,
    descent_rate: Option<f64>,
    descent_fuel_flow: Option<f64>,
}

impl RawPerf {
    fn new() -> Self {
        Self {
            name: None,
            aircraft_type: None,
            fuel_as_volume: false,
            jet_fuel: true,
            usable_fuel: None,
            min_runway_length: None,
            runway_type: None,
            climb_speed: None,
            climb_rate: None,
            climb_fuel_flow: None,
            cruise_speed: None,
            cruise_fuel_flow: None,
            descent_speed: None,
            descent_rate: None,
            descent_fuel_flow: None,
        }
    }
}

fn parse_xml(contents: &str) -> Result<LnmImportResult, String> {
    let mut reader = Reader::from_str(contents);
    let mut path: Vec<String> = Vec::new();
    let mut raw = RawPerf::new();
    let mut current_text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                path.push(tag);
                current_text.clear();
            }
            Ok(Event::Text(e)) => {
                current_text = e.unescape().unwrap_or_default().to_string();
            }
            Ok(Event::End(_)) => {
                let full_path = path.join("/");
                let val = current_text.trim().to_string();

                if full_path.ends_with("Options/Name") {
                    raw.name = Some(val);
                } else if full_path.ends_with("Options/AircraftType") {
                    raw.aircraft_type = Some(val);
                } else if full_path.ends_with("Options/FuelAsVolume") {
                    raw.fuel_as_volume = val == "1" || val.eq_ignore_ascii_case("true");
                } else if full_path.ends_with("Options/JetFuel") {
                    raw.jet_fuel = val == "1" || val.eq_ignore_ascii_case("true");
                } else if full_path.ends_with("Perf/UsableFuelLbsGal") {
                    raw.usable_fuel = val.parse().ok();
                } else if full_path.ends_with("Perf/MinRunwayLengthFt") {
                    raw.min_runway_length = val.parse().ok();
                } else if full_path.ends_with("Perf/RunwayType") {
                    raw.runway_type = Some(val);
                } else if full_path.ends_with("Climb/SpeedKtsTAS") {
                    raw.climb_speed = val.parse().ok();
                } else if full_path.ends_with("Climb/VertSpeedFtPerMin") {
                    raw.climb_rate = val.parse().ok();
                } else if full_path.ends_with("Climb/FuelFlowLbsGalPerHour") {
                    raw.climb_fuel_flow = val.parse().ok();
                } else if full_path.ends_with("Cruise/SpeedKtsTAS") {
                    raw.cruise_speed = val.parse().ok();
                } else if full_path.ends_with("Cruise/FuelFlowLbsGalPerHour") {
                    raw.cruise_fuel_flow = val.parse().ok();
                } else if full_path.ends_with("Descent/SpeedKtsTAS") {
                    raw.descent_speed = val.parse().ok();
                } else if full_path.ends_with("Descent/VertSpeedFtPerMin") {
                    raw.descent_rate = val.parse().ok();
                } else if full_path.ends_with("Descent/FuelFlowLbsGalPerHour") {
                    raw.descent_fuel_flow = val.parse().ok();
                }

                current_text.clear();
                path.pop();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
    }

    build_toml(raw)
}

fn parse_ini(contents: &str) -> Result<LnmImportResult, String> {
    let mut raw = RawPerf::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') || line.starts_with('#') {
            continue;
        }

        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let val = val.trim();

        match key {
            "Name" => raw.name = Some(val.to_string()),
            "AircraftType" => raw.aircraft_type = Some(val.to_string()),
            "FuelAsVolume" => {
                raw.fuel_as_volume = val == "1" || val.eq_ignore_ascii_case("true");
            }
            "JetFuel" => {
                raw.jet_fuel = val == "1" || val.eq_ignore_ascii_case("true");
            }
            "UsableFuel" | "UsableFuelLbsGal" => raw.usable_fuel = val.parse().ok(),
            "MinRunwayLength" | "MinRunwayLengthFt" => raw.min_runway_length = val.parse().ok(),
            "RunwayType" => raw.runway_type = Some(val.to_string()),
            "ClimbSpeedKtsTAS" => raw.climb_speed = val.parse().ok(),
            "ClimbVertSpeedFtPerMin" => raw.climb_rate = val.parse().ok(),
            "ClimbFuelFlowLbsGalPerHour" => raw.climb_fuel_flow = val.parse().ok(),
            "CruiseSpeedKtsTAS" => raw.cruise_speed = val.parse().ok(),
            "CruiseFuelFlowLbsGalPerHour" => raw.cruise_fuel_flow = val.parse().ok(),
            "DescentSpeedKtsTAS" => raw.descent_speed = val.parse().ok(),
            "DescentVertSpeedFtPerMin" => raw.descent_rate = val.parse().ok(),
            "DescentFuelFlowLbsGalPerHour" => raw.descent_fuel_flow = val.parse().ok(),
            _ => {}
        }
    }

    build_toml(raw)
}

fn build_toml(raw: RawPerf) -> Result<LnmImportResult, String> {
    let mut warnings = Vec::new();

    let aircraft_type = raw
        .aircraft_type
        .filter(|s| !s.is_empty())
        .ok_or("missing AircraftType in LNM file")?;

    let name = raw
        .name
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| aircraft_type.clone());

    let fuel_type_str = if raw.jet_fuel { "jet" } else { "avgas" };

    // Convert fuel values to kg
    let convert_fuel = |lbs_or_gal: f64| -> f64 {
        if raw.fuel_as_volume {
            gal_to_kg(lbs_or_gal, raw.jet_fuel)
        } else {
            lbs_to_kg(lbs_or_gal)
        }
    };

    let warn_missing = |field: &str, warnings: &mut Vec<String>| {
        warnings.push(format!("{field} missing from source file; defaulting to 0"));
    };

    let capacity_kg = raw.usable_fuel.map(convert_fuel).unwrap_or_else(|| {
        warn_missing("usable fuel", &mut warnings);
        0.0
    });

    let climb_fuel_kg = raw.climb_fuel_flow.map(convert_fuel).unwrap_or(0.0);
    let cruise_fuel_kg = raw.cruise_fuel_flow.map(convert_fuel).unwrap_or(0.0);
    let descent_fuel_kg = raw.descent_fuel_flow.map(convert_fuel).unwrap_or(0.0);

    let cruise_speed = raw.cruise_speed.unwrap_or_else(|| {
        warn_missing("cruise speed", &mut warnings);
        0.0
    }).round() as u16;
    let climb_speed = raw.climb_speed.unwrap_or(0.0).round() as u16;
    let climb_rate = raw.climb_rate.unwrap_or(0.0).round() as u16;
    let descent_speed = raw.descent_speed.unwrap_or(0.0).round() as u16;
    let descent_rate = raw.descent_rate.unwrap_or(0.0).round() as u16;
    let min_runway = raw.min_runway_length.unwrap_or(0.0).round() as u32;

    // Estimate cruise altitude from cruise speed heuristic
    let cruise_altitude_ft: u32 = if cruise_speed > 300 {
        35000
    } else if cruise_speed > 200 {
        25000
    } else if cruise_speed > 150 {
        12000
    } else {
        8000
    };
    warnings.push(format!(
        "cruise_altitude_ft estimated as {cruise_altitude_ft} from cruise speed; adjust if needed"
    ));

    let service_ceiling_ft: u32 = if cruise_altitude_ft >= 35000 {
        41000
    } else if cruise_altitude_ft >= 25000 {
        35000
    } else if cruise_altitude_ft >= 12000 {
        20000
    } else {
        14000
    };
    warnings.push(format!(
        "service_ceiling_ft estimated as {service_ceiling_ft}; adjust if needed"
    ));

    let toml_content = format!(
        r#"[aircraft]
name = "{name}"
icao_type = "{aircraft_type}"

[performance]
cruise_speed_ktas = {cruise_speed}
cruise_altitude_ft = {cruise_altitude_ft}
service_ceiling_ft = {service_ceiling_ft}
min_runway_length_ft = {min_runway}

[performance.climb]
speed_ktas = {climb_speed}
rate_fpm = {climb_rate}

[performance.descent]
speed_ktas = {descent_speed}
rate_fpm = {descent_rate}

[fuel]
capacity_kg = {capacity_kg:.1}
fuel_type = "{fuel_type_str}"

[fuel.flow]
climb_kg_per_hour = {climb_fuel_kg:.1}
cruise_kg_per_hour = {cruise_fuel_kg:.1}
descent_kg_per_hour = {descent_fuel_kg:.1}
"#
    );

    Ok(LnmImportResult {
        icao_type: aircraft_type,
        toml_content,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_xml_format() {
        let path = Path::new("tests/fixtures/sample.lnmperf");
        let result = import_lnmperf(path).expect("should parse XML");
        assert!(
            result.toml_content.contains("icao_type = \"B737\""),
            "should contain B737 type"
        );
        assert!(
            result.toml_content.contains("cruise_speed_ktas = 375"),
            "should contain cruise speed 375"
        );
        // 46063 lbs * 0.453592 = ~20893 kg
        assert!(
            result.toml_content.contains("20893"),
            "should contain capacity ~20893 kg, got:\n{}",
            result.toml_content
        );
    }

    #[test]
    fn import_ini_format() {
        let path = Path::new("tests/fixtures/sample_legacy.lnmperf");
        let result = import_lnmperf(path).expect("should parse INI");
        assert!(
            result.toml_content.contains("icao_type = \"C152\""),
            "should contain C152 type"
        );
        assert!(
            result.toml_content.contains("cruise_speed_ktas = 103"),
            "should contain cruise speed 103"
        );
        // 26 gal * 2.7216 = ~70.76 kg
        assert!(
            result.toml_content.contains("capacity_kg = 70."),
            "should contain capacity ~70 kg, got:\n{}",
            result.toml_content
        );
        assert!(
            result.toml_content.contains("fuel_type = \"avgas\""),
            "should contain avgas fuel type"
        );
    }

    #[test]
    fn import_missing_aircraft_type() {
        let tmp_path = Path::new("/tmp/claude-1000/test_missing_type.lnmperf");
        std::fs::create_dir_all("/tmp/claude-1000").ok();
        std::fs::write(
            tmp_path,
            "[Options]\nName=NoType\n\n[Perf]\nCruiseSpeedKtsTAS=100\n",
        )
        .expect("write tmp file");
        let result = import_lnmperf(tmp_path);
        assert!(result.is_err(), "should error on missing AircraftType");
        assert!(
            result.unwrap_err().contains("AircraftType"),
            "error should mention AircraftType"
        );
    }
}
