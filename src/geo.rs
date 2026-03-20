// Haversine distance calculation

/// Calculates great-circle distance in nautical miles between two points.
pub fn haversine_distance_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_NM: f64 = 3440.065; // mean Earth radius in nautical miles

    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();
    let dlat = lat2 - lat1;
    let dlon = (lon2 - lon1).to_radians();

    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_NM * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_distance_jfk_to_lhr() {
        // JFK (40.6413, -73.7781) to LHR (51.4700, -0.4543)
        // Known great-circle distance: ~2999 nm
        let d = haversine_distance_nm(40.6413, -73.7781, 51.4700, -0.4543);
        assert!((d - 2999.0).abs() < 10.0, "JFK-LHR distance was {d}, expected ~2999 nm");
    }

    #[test]
    fn known_distance_sfo_to_nrt() {
        // SFO (37.6213, -122.3790) to NRT (35.7647, 140.3864)
        // Haversine great-circle distance: ~4442 nm
        let d = haversine_distance_nm(37.6213, -122.3790, 35.7647, 140.3864);
        assert!((d - 4442.0).abs() < 10.0, "SFO-NRT distance was {d}, expected ~4442 nm");
    }

    #[test]
    fn zero_distance_same_point() {
        let d = haversine_distance_nm(51.4700, -0.4543, 51.4700, -0.4543);
        assert!(d.abs() < 0.01, "Same point distance should be ~0, was {d}");
    }

    #[test]
    fn short_distance_eddf_to_eddm() {
        // Frankfurt (50.0379, 8.5622) to Munich (48.3537, 11.7750)
        // Haversine great-circle distance: ~162 nm
        let d = haversine_distance_nm(50.0379, 8.5622, 48.3537, 11.7750);
        assert!((d - 162.0).abs() < 5.0, "EDDF-EDDM distance was {d}, expected ~162 nm");
    }
}
