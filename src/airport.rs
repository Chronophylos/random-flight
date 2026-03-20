#[derive(Debug, Clone)]
pub struct Airport {
    pub icao: &'static str,
    pub name: &'static str,
    pub latitude: f64,
    pub longitude: f64,
    pub elevation_ft: i32,
    pub runway_length_ft: u32,
}

include!(concat!(env!("OUT_DIR"), "/airport_db.rs"));

pub fn all_airports() -> &'static [Airport] {
    AIRPORTS
}

pub fn find_by_icao(icao: &str) -> Option<&'static Airport> {
    AIRPORTS.iter().find(|a| a.icao.eq_ignore_ascii_case(icao))
}

pub fn filter_by_runway(min_length_ft: u32) -> Vec<&'static Airport> {
    AIRPORTS
        .iter()
        .filter(|a| a.runway_length_ft >= min_length_ft)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_has_airports() {
        assert!(all_airports().len() > 1000, "Expected >1000 airports, got {}", all_airports().len());
    }

    #[test]
    fn find_known_airport() {
        let apt = find_by_icao("KJFK").expect("KJFK should exist");
        assert_eq!(apt.icao, "KJFK");
        assert!((apt.latitude - 40.64).abs() < 0.1);
    }

    #[test]
    fn find_case_insensitive() {
        assert!(find_by_icao("kjfk").is_some());
    }

    #[test]
    fn find_unknown_returns_none() {
        assert!(find_by_icao("ZZZZ").is_none());
    }

    #[test]
    fn filter_by_runway_reduces_count() {
        let all = all_airports().len();
        let long_only = filter_by_runway(8000);
        assert!(long_only.len() < all);
        assert!(long_only.len() > 10, "Should have some airports with 8000ft+ runways");
        for a in &long_only {
            assert!(a.runway_length_ft >= 8000);
        }
    }
}
