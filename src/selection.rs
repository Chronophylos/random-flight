use std::time::Duration;

use rand::Rng;

use crate::aircraft::Aircraft;
use crate::airport::{self, Airport};
use crate::error::Error;
use crate::flight_plan::{calculate_flight_plan, estimate_distance_for_block_time, FlightPlan};
use crate::geo::haversine_distance_nm;

pub struct FlightPlanOptions {
    pub tolerance: Duration,
    pub taxi_time: Duration,
    pub max_retries: u32,
    pub departure_icao: Option<String>,
    pub arrival_icao: Option<String>,
}

impl Default for FlightPlanOptions {
    fn default() -> Self {
        Self {
            tolerance: Duration::from_secs(15 * 60),
            taxi_time: Duration::from_secs(10 * 60),
            max_retries: 100,
            departure_icao: None,
            arrival_icao: None,
        }
    }
}

pub fn generate_flight_plan(
    aircraft: &Aircraft,
    target_block_time: Duration,
    options: Option<FlightPlanOptions>,
) -> Result<FlightPlan, Error> {
    let mut rng = rand::rng();
    generate_flight_plan_with_rng(aircraft, target_block_time, options, &mut rng)
}

pub fn generate_flight_plan_with_rng(
    aircraft: &Aircraft,
    target_block_time: Duration,
    options: Option<FlightPlanOptions>,
    rng: &mut impl Rng,
) -> Result<FlightPlan, Error> {
    let opts = options.unwrap_or_default();

    // Handle pinned airports
    if let (Some(dep_icao), Some(arr_icao)) = (&opts.departure_icao, &opts.arrival_icao) {
        return plan_for_pair(dep_icao, arr_icao, aircraft, opts.taxi_time);
    }

    let eligible = airport::filter_by_runway(aircraft.min_runway_length_ft);
    if eligible.is_empty() {
        return Err(Error::NoValidAirports);
    }

    let target_dist = estimate_distance_for_block_time(aircraft, target_block_time, opts.taxi_time);
    let tolerance_hrs = opts.tolerance.as_secs_f64() / 3600.0;
    let dist_margin = aircraft.cruise_speed_kts as f64 * tolerance_hrs;

    let min_dist = (target_dist - dist_margin).max(1.0);
    let max_dist = target_dist + dist_margin;

    let pinned_departure = opts.departure_icao.is_some();

    for _attempt in 0..opts.max_retries {
        let departure = pick_departure(&eligible, &opts, aircraft, rng)?;

        let candidates: Vec<&'static Airport> = eligible
            .iter()
            .copied()
            .filter(|a| {
                if std::ptr::eq(*a, departure) {
                    return false;
                }
                let d = haversine_distance_nm(
                    departure.latitude, departure.longitude,
                    a.latitude, a.longitude,
                );
                d >= min_dist && d <= max_dist && d <= aircraft.range_nm as f64
            })
            .collect();

        if candidates.is_empty() {
            // If departure is pinned, retrying won't help — different error
            if pinned_departure {
                return Err(Error::NoCandidateArrivals);
            }
            continue;
        }

        let arrival = candidates[rng.random_range(0..candidates.len())];
        let fp = calculate_flight_plan(departure, arrival, aircraft, opts.taxi_time);

        if fp.block_time.abs_diff(target_block_time) <= opts.tolerance {
            return Ok(fp);
        }
    }

    Err(Error::RetriesExhausted { attempts: opts.max_retries })
}

fn pick_departure(
    eligible: &[&'static Airport],
    opts: &FlightPlanOptions,
    aircraft: &Aircraft,
    rng: &mut impl Rng,
) -> Result<&'static Airport, Error> {
    if let Some(icao) = &opts.departure_icao {
        let apt = airport::find_by_icao(icao)
            .ok_or_else(|| Error::UnknownAirport { icao: icao.clone() })?;
        if apt.runway_length_ft < aircraft.min_runway_length_ft {
            return Err(Error::RunwayTooShort {
                airport_icao: icao.clone(),
                required_ft: aircraft.min_runway_length_ft,
                available_ft: apt.runway_length_ft,
            });
        }
        Ok(apt)
    } else {
        Ok(eligible[rng.random_range(0..eligible.len())])
    }
}

fn plan_for_pair(
    dep_icao: &str,
    arr_icao: &str,
    aircraft: &Aircraft,
    taxi_time: Duration,
) -> Result<FlightPlan, Error> {
    let dep = airport::find_by_icao(dep_icao)
        .ok_or_else(|| Error::UnknownAirport { icao: dep_icao.to_string() })?;
    let arr = airport::find_by_icao(arr_icao)
        .ok_or_else(|| Error::UnknownAirport { icao: arr_icao.to_string() })?;

    if dep.runway_length_ft < aircraft.min_runway_length_ft {
        return Err(Error::RunwayTooShort {
            airport_icao: dep_icao.to_string(),
            required_ft: aircraft.min_runway_length_ft,
            available_ft: dep.runway_length_ft,
        });
    }
    if arr.runway_length_ft < aircraft.min_runway_length_ft {
        return Err(Error::RunwayTooShort {
            airport_icao: arr_icao.to_string(),
            required_ft: aircraft.min_runway_length_ft,
            available_ft: arr.runway_length_ft,
        });
    }

    let distance = haversine_distance_nm(
        dep.latitude, dep.longitude, arr.latitude, arr.longitude,
    );
    if distance > aircraft.range_nm as f64 {
        return Err(Error::RangeExceeded {
            distance_nm: distance,
            range_nm: aircraft.range_nm,
        });
    }

    Ok(calculate_flight_plan(dep, arr, aircraft, taxi_time))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aircraft::aircraft_by_name;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    fn seeded_rng() -> SmallRng {
        SmallRng::seed_from_u64(42)
    }

    #[test]
    fn generates_plan_within_tolerance() {
        let ac = aircraft_by_name("B738").expect("B738");
        let target = Duration::from_secs(2 * 3600);
        let opts = FlightPlanOptions {
            tolerance: Duration::from_secs(15 * 60),
            ..Default::default()
        };

        let mut rng = seeded_rng();
        let fp = generate_flight_plan_with_rng(ac, target, Some(opts), &mut rng)
            .expect("should find a flight");

        let diff = if fp.block_time > target {
            fp.block_time - target
        } else {
            target - fp.block_time
        };
        assert!(diff <= Duration::from_secs(15 * 60),
            "block time {} min not within tolerance of target 120 min",
            fp.block_time.as_secs() / 60);
    }

    #[test]
    fn pinned_both_airports() {
        let ac = aircraft_by_name("B738").expect("B738");
        let target = Duration::from_secs(5 * 3600); // doesn't matter for pinned
        let opts = FlightPlanOptions {
            departure_icao: Some("KJFK".into()),
            arrival_icao: Some("KLAX".into()),
            ..Default::default()
        };

        let mut rng = seeded_rng();
        let fp = generate_flight_plan_with_rng(ac, target, Some(opts), &mut rng)
            .expect("should compute pinned plan");

        assert_eq!(fp.departure.icao, "KJFK");
        assert_eq!(fp.arrival.icao, "KLAX");
    }

    #[test]
    fn unknown_airport_error() {
        let ac = aircraft_by_name("C172").expect("C172");
        let opts = FlightPlanOptions {
            departure_icao: Some("ZZZZ".into()),
            arrival_icao: Some("KJFK".into()),
            ..Default::default()
        };

        let mut rng = seeded_rng();
        let result = generate_flight_plan_with_rng(ac, Duration::from_secs(3600), Some(opts), &mut rng);
        assert!(matches!(result, Err(Error::UnknownAirport { .. })));
    }

    #[test]
    fn range_exceeded_error() {
        let ac = aircraft_by_name("C172").expect("C172"); // 640 nm range
        let opts = FlightPlanOptions {
            departure_icao: Some("KJFK".into()),
            arrival_icao: Some("EGLL".into()), // ~2999 nm
            ..Default::default()
        };

        let mut rng = seeded_rng();
        let result = generate_flight_plan_with_rng(ac, Duration::from_secs(3600), Some(opts), &mut rng);
        assert!(matches!(result, Err(Error::RangeExceeded { .. })));
    }

    #[test]
    fn pinned_departure_random_arrival() {
        let ac = aircraft_by_name("B738").expect("B738");
        let target = Duration::from_secs(2 * 3600);
        let opts = FlightPlanOptions {
            departure_icao: Some("EDDF".into()),
            tolerance: Duration::from_secs(15 * 60),
            ..Default::default()
        };

        let mut rng = seeded_rng();
        let fp = generate_flight_plan_with_rng(ac, target, Some(opts), &mut rng)
            .expect("should find a flight from EDDF");

        assert_eq!(fp.departure.icao, "EDDF");
    }
}
