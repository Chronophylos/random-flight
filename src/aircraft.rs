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
    BUILT_IN
}

pub fn aircraft_by_icao_type(icao_type: &str) -> Option<&'static Aircraft> {
    BUILT_IN.iter().find(|a| a.icao_type.eq_ignore_ascii_case(icao_type))
}

#[deprecated(note = "use aircraft_by_icao_type instead")]
pub fn aircraft_by_name(name: &str) -> Option<&'static Aircraft> {
    aircraft_by_icao_type(name)
}

static BUILT_IN: &[Aircraft] = &[
    Aircraft {
        name: "Cessna 172 Skyhawk",
        icao_type: "C172",
        cruise_speed_ktas: 122,
        cruise_altitude_ft: 8000,
        service_ceiling_ft: 14000,
        min_runway_length_ft: 2000,
        climb_speed_ktas: 80,
        climb_rate_fpm: 700,
        descent_speed_ktas: 90,
        descent_rate_fpm: 500,
        fuel_capacity_kg: 109.0,
        fuel_type: FuelType::Avgas,
        fuel_flow_climb_kg_per_hour: 30.0,
        fuel_flow_cruise_kg_per_hour: 21.0,
        fuel_flow_descent_kg_per_hour: 16.0,
    },
    Aircraft {
        name: "Cessna 208B Grand Caravan",
        icao_type: "C208",
        cruise_speed_ktas: 186,
        cruise_altitude_ft: 14000,
        service_ceiling_ft: 25000,
        min_runway_length_ft: 3000,
        climb_speed_ktas: 140,
        climb_rate_fpm: 1000,
        descent_speed_ktas: 120,
        descent_rate_fpm: 800,
        fuel_capacity_kg: 1000.0,
        fuel_type: FuelType::Jet,
        fuel_flow_climb_kg_per_hour: 200.0,
        fuel_flow_cruise_kg_per_hour: 160.0,
        fuel_flow_descent_kg_per_hour: 100.0,
    },
    Aircraft {
        name: "Boeing 737-800",
        icao_type: "B738",
        cruise_speed_ktas: 460,
        cruise_altitude_ft: 36000,
        service_ceiling_ft: 41000,
        min_runway_length_ft: 6000,
        climb_speed_ktas: 310,
        climb_rate_fpm: 2500,
        descent_speed_ktas: 280,
        descent_rate_fpm: 1800,
        fuel_capacity_kg: 20894.0,
        fuel_type: FuelType::Jet,
        fuel_flow_climb_kg_per_hour: 3402.0,
        fuel_flow_cruise_kg_per_hour: 2359.0,
        fuel_flow_descent_kg_per_hour: 1270.0,
    },
    Aircraft {
        name: "Airbus A320",
        icao_type: "A320",
        cruise_speed_ktas: 447,
        cruise_altitude_ft: 36000,
        service_ceiling_ft: 41000,
        min_runway_length_ft: 6000,
        climb_speed_ktas: 310,
        climb_rate_fpm: 2500,
        descent_speed_ktas: 280,
        descent_rate_fpm: 1800,
        fuel_capacity_kg: 19144.0,
        fuel_type: FuelType::Jet,
        fuel_flow_climb_kg_per_hour: 3300.0,
        fuel_flow_cruise_kg_per_hour: 2300.0,
        fuel_flow_descent_kg_per_hour: 1200.0,
    },
    Aircraft {
        name: "Airbus A380-800",
        icao_type: "A388",
        cruise_speed_ktas: 480,
        cruise_altitude_ft: 40000,
        service_ceiling_ft: 45000,
        min_runway_length_ft: 9000,
        climb_speed_ktas: 330,
        climb_rate_fpm: 2000,
        descent_speed_ktas: 300,
        descent_rate_fpm: 1500,
        fuel_capacity_kg: 253983.0,
        fuel_type: FuelType::Jet,
        fuel_flow_climb_kg_per_hour: 14000.0,
        fuel_flow_cruise_kg_per_hour: 10000.0,
        fuel_flow_descent_kg_per_hour: 5500.0,
    },
    Aircraft {
        name: "Bombardier CRJ-700",
        icao_type: "CRJ7",
        cruise_speed_ktas: 447,
        cruise_altitude_ft: 37000,
        service_ceiling_ft: 41000,
        min_runway_length_ft: 5500,
        climb_speed_ktas: 310,
        climb_rate_fpm: 2500,
        descent_speed_ktas: 280,
        descent_rate_fpm: 1800,
        fuel_capacity_kg: 8875.0,
        fuel_type: FuelType::Jet,
        fuel_flow_climb_kg_per_hour: 2200.0,
        fuel_flow_cruise_kg_per_hour: 1600.0,
        fuel_flow_descent_kg_per_hour: 900.0,
    },
];

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
