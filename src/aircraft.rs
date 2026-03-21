#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuelType {
    Jet,   // Jet-A, density 0.804 kg/L
    Avgas, // 100LL, density 0.721 kg/L
}

#[derive(Debug, Clone)]
pub struct Aircraft {
    pub name: &'static str,
    pub icao_type: &'static str,
    pub cruise_speed_ktas: u16,
    pub cruise_altitude_ft: u32,
    pub service_ceiling_ft: u32,
    pub min_runway_length_ft: u32,
    pub climb_speed_ktas: u16,
    pub climb_rate_fpm: u16,
    pub descent_speed_ktas: u16,
    pub descent_rate_fpm: u16,
    pub fuel_capacity_kg: f64,
    pub fuel_type: FuelType,
    pub fuel_flow_climb_kg_per_hour: f64,
    pub fuel_flow_cruise_kg_per_hour: f64,
    pub fuel_flow_descent_kg_per_hour: f64,
}

impl Aircraft {
    /// Derived max range with 5% fuel reserve.
    pub fn range_nm(&self) -> f64 {
        (self.fuel_capacity_kg * 0.95 / self.fuel_flow_cruise_kg_per_hour)
            * self.cruise_speed_ktas as f64
    }
}

pub fn built_in_aircraft() -> &'static [Aircraft] {
    AIRCRAFT_DB
}

pub fn aircraft_by_icao_type(icao_type: &str) -> Option<&'static Aircraft> {
    AIRCRAFT_DB.iter().find(|a| a.icao_type.eq_ignore_ascii_case(icao_type))
}

include!(concat!(env!("OUT_DIR"), "/aircraft_db.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c172_preset_exists() {
        let a = aircraft_by_icao_type("C172").expect("C172 should exist");
        assert_eq!(a.cruise_speed_ktas, 122);
        assert_eq!(a.min_runway_length_ft, 2000);
    }

    #[test]
    fn b738_preset_exists() {
        let a = aircraft_by_icao_type("B738").expect("B738 should exist");
        assert!(a.cruise_speed_ktas > 400);
        assert!(a.range_nm() > 2000.0);
    }

    #[test]
    fn range_derived_from_fuel() {
        let a = aircraft_by_icao_type("B738").expect("B738");
        let expected = (a.fuel_capacity_kg * 0.95 / a.fuel_flow_cruise_kg_per_hour)
            * a.cruise_speed_ktas as f64;
        assert!((a.range_nm() - expected).abs() < 0.01);
    }

    #[test]
    fn case_insensitive_lookup() {
        assert!(aircraft_by_icao_type("c172").is_some());
        assert!(aircraft_by_icao_type("C172").is_some());
    }

    #[test]
    fn unknown_aircraft_returns_none() {
        assert!(aircraft_by_icao_type("ZZZZ").is_none());
    }

    #[test]
    fn built_in_has_entries() {
        assert!(built_in_aircraft().len() >= 4);
    }
}
