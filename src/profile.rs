use std::path::Path;

use serde::Deserialize;

use crate::aircraft::{Aircraft, FuelType};

#[derive(Debug, Deserialize)]
struct AircraftToml {
    aircraft: AircraftMeta,
    performance: PerformanceToml,
    fuel: FuelToml,
}

#[derive(Debug, Deserialize)]
struct AircraftMeta {
    name: String,
    icao_type: String,
}

#[derive(Debug, Deserialize)]
struct PerformanceToml {
    cruise_speed_ktas: u16,
    cruise_altitude_ft: u32,
    service_ceiling_ft: u32,
    min_runway_length_ft: u32,
    climb: PhaseToml,
    descent: PhaseToml,
}

#[derive(Debug, Deserialize)]
struct PhaseToml {
    speed_ktas: u16,
    rate_fpm: u16,
}

#[derive(Debug, Deserialize)]
struct FuelToml {
    capacity_kg: f64,
    fuel_type: String,
    flow: FuelFlowToml,
}

#[derive(Debug, Deserialize)]
struct FuelFlowToml {
    climb_kg_per_hour: f64,
    cruise_kg_per_hour: f64,
    descent_kg_per_hour: f64,
}

pub fn load_profile(path: &Path) -> Result<Aircraft, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    parse_profile(&contents)
}

fn parse_profile(contents: &str) -> Result<Aircraft, String> {
    let parsed: AircraftToml = toml::from_str(contents)
        .map_err(|e| format!("failed to parse TOML: {e}"))?;

    let fuel_type = match parsed.fuel.fuel_type.as_str() {
        "jet" => FuelType::Jet,
        "avgas" => FuelType::Avgas,
        other => return Err(format!("unknown fuel_type: {other}")),
    };

    if parsed.performance.service_ceiling_ft < parsed.performance.cruise_altitude_ft {
        return Err("service_ceiling_ft must be >= cruise_altitude_ft".into());
    }

    // Leak strings to get 'static lifetime — acceptable for one-time startup load
    let name: &'static str = Box::leak(parsed.aircraft.name.into_boxed_str());
    let icao_type: &'static str = Box::leak(parsed.aircraft.icao_type.into_boxed_str());

    Ok(Aircraft {
        name,
        icao_type,
        cruise_speed_ktas: parsed.performance.cruise_speed_ktas,
        cruise_altitude_ft: parsed.performance.cruise_altitude_ft,
        service_ceiling_ft: parsed.performance.service_ceiling_ft,
        min_runway_length_ft: parsed.performance.min_runway_length_ft,
        climb_speed_ktas: parsed.performance.climb.speed_ktas,
        climb_rate_fpm: parsed.performance.climb.rate_fpm,
        descent_speed_ktas: parsed.performance.descent.speed_ktas,
        descent_rate_fpm: parsed.performance.descent.rate_fpm,
        fuel_capacity_kg: parsed.fuel.capacity_kg,
        fuel_type,
        fuel_flow_climb_kg_per_hour: parsed.fuel.flow.climb_kg_per_hour,
        fuel_flow_cruise_kg_per_hour: parsed.fuel.flow.cruise_kg_per_hour,
        fuel_flow_descent_kg_per_hour: parsed.fuel.flow.descent_kg_per_hour,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
[aircraft]
name = "Test Aircraft"
icao_type = "TEST"

[performance]
cruise_speed_ktas = 250
cruise_altitude_ft = 25000
service_ceiling_ft = 30000
min_runway_length_ft = 4000

[performance.climb]
speed_ktas = 200
rate_fpm = 2000

[performance.descent]
speed_ktas = 180
rate_fpm = 1500

[fuel]
capacity_kg = 5000.0
fuel_type = "jet"

[fuel.flow]
climb_kg_per_hour = 1500.0
cruise_kg_per_hour = 1000.0
descent_kg_per_hour = 600.0
"#;

    #[test]
    fn parse_valid_profile() {
        let ac = parse_profile(VALID_TOML).expect("should parse");
        assert_eq!(ac.icao_type, "TEST");
        assert_eq!(ac.cruise_speed_ktas, 250);
        assert_eq!(ac.fuel_type, FuelType::Jet);
        assert!((ac.fuel_capacity_kg - 5000.0).abs() < 0.01);
    }

    #[test]
    fn parse_invalid_fuel_type() {
        let toml = VALID_TOML.replace("\"jet\"", "\"diesel\"");
        let result = parse_profile(&toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fuel_type"));
    }

    #[test]
    fn parse_ceiling_below_cruise() {
        let toml = VALID_TOML.replace("service_ceiling_ft = 30000", "service_ceiling_ft = 20000");
        let result = parse_profile(&toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ceiling"));
    }

    #[test]
    fn parse_missing_field() {
        let toml = "[aircraft]\nname = \"X\"\nicao_type = \"X\"\n";
        let result = parse_profile(toml);
        assert!(result.is_err());
    }

    #[test]
    fn round_trip_matches_built_in() {
        let path = std::path::Path::new("data/aircraft/b738.toml");
        if path.exists() {
            let loaded = load_profile(path).expect("should load b738.toml");
            let built_in = crate::aircraft::aircraft_by_icao_type("B738").expect("B738");
            assert_eq!(loaded.cruise_speed_ktas, built_in.cruise_speed_ktas);
            assert_eq!(loaded.climb_rate_fpm, built_in.climb_rate_fpm);
            assert!((loaded.fuel_capacity_kg - built_in.fuel_capacity_kg).abs() < 0.1);
        }
    }
}
