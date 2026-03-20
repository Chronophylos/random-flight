#[derive(Debug, Clone)]
pub struct Aircraft {
    pub name: &'static str,
    pub cruise_speed_kts: u16,
    pub cruise_altitude_ft: u32,
    pub climb_rate_fpm: u16,
    pub descent_rate_fpm: u16,
    pub climb_speed_factor: f32,
    pub descent_speed_factor: f32,
    pub range_nm: u32,
    pub min_runway_length_ft: u32,
}

pub fn built_in_aircraft() -> &'static [Aircraft] {
    BUILT_IN
}

pub fn aircraft_by_name(name: &str) -> Option<&'static Aircraft> {
    let name_upper = name.to_uppercase();
    BUILT_IN.iter().find(|a| a.name.to_uppercase() == name_upper)
}

static BUILT_IN: &[Aircraft] = &[
    Aircraft {
        name: "C172",
        cruise_speed_kts: 122,
        cruise_altitude_ft: 8000,
        climb_rate_fpm: 700,
        descent_rate_fpm: 500,
        climb_speed_factor: 0.75,
        descent_speed_factor: 0.65,
        range_nm: 640,
        min_runway_length_ft: 2000,
    },
    Aircraft {
        name: "C208",
        cruise_speed_kts: 186,
        cruise_altitude_ft: 14000,
        climb_rate_fpm: 1000,
        descent_rate_fpm: 800,
        climb_speed_factor: 0.75,
        descent_speed_factor: 0.65,
        range_nm: 900,
        min_runway_length_ft: 3000,
    },
    Aircraft {
        name: "B738",
        cruise_speed_kts: 460,
        cruise_altitude_ft: 36000,
        climb_rate_fpm: 2500,
        descent_rate_fpm: 1800,
        climb_speed_factor: 0.75,
        descent_speed_factor: 0.65,
        range_nm: 2935,
        min_runway_length_ft: 6000,
    },
    Aircraft {
        name: "A320",
        cruise_speed_kts: 447,
        cruise_altitude_ft: 36000,
        climb_rate_fpm: 2500,
        descent_rate_fpm: 1800,
        climb_speed_factor: 0.75,
        descent_speed_factor: 0.65,
        range_nm: 3300,
        min_runway_length_ft: 6000,
    },
    Aircraft {
        name: "A388",
        cruise_speed_kts: 480,
        cruise_altitude_ft: 40000,
        climb_rate_fpm: 2000,
        descent_rate_fpm: 1500,
        climb_speed_factor: 0.75,
        descent_speed_factor: 0.65,
        range_nm: 8000,
        min_runway_length_ft: 9000,
    },
    Aircraft {
        name: "CRJ7",
        cruise_speed_kts: 447,
        cruise_altitude_ft: 37000,
        climb_rate_fpm: 2500,
        descent_rate_fpm: 1800,
        climb_speed_factor: 0.75,
        descent_speed_factor: 0.65,
        range_nm: 1350,
        min_runway_length_ft: 5500,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c172_preset_exists() {
        let a = aircraft_by_name("C172").expect("C172 should exist");
        assert_eq!(a.cruise_speed_kts, 122);
        assert_eq!(a.min_runway_length_ft, 2000);
    }

    #[test]
    fn b738_preset_exists() {
        let a = aircraft_by_name("B738").expect("B738 should exist");
        assert!(a.cruise_speed_kts > 400);
        assert!(a.range_nm > 2000);
    }

    #[test]
    fn case_insensitive_lookup() {
        assert!(aircraft_by_name("c172").is_some());
        assert!(aircraft_by_name("C172").is_some());
    }

    #[test]
    fn unknown_aircraft_returns_none() {
        assert!(aircraft_by_name("ZZZZ").is_none());
    }

    #[test]
    fn built_in_has_entries() {
        assert!(built_in_aircraft().len() >= 4);
    }
}
