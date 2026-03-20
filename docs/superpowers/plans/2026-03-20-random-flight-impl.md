# Random Flight Plan Generator — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust crate that generates random flight plans matching a target block time, with a CLI interface.

**Architecture:** Bottom-up build starting with pure math (geo), data types (aircraft), build-time codegen (airport DB), flight model (block time calc), selection algorithm, public API, and CLI. Each layer depends only on layers below it.

**Tech Stack:** Rust (edition 2024), clap, rand, serde, csv, thiserror, humantime, std::time::Duration

**Spec:** `docs/superpowers/specs/2026-03-19-random-flight-design.md`

---

## File Structure

```
random-flight/
├── Cargo.toml              # crate manifest, dependencies, build-dependencies
├── build.rs                # downloads OurAirports CSVs, generates airport_db.rs
├── src/
│   ├── lib.rs              # public API: generate_flight_plan, re-exports
│   ├── geo.rs              # haversine_distance_nm()
│   ├── aircraft.rs         # Aircraft struct, built-in presets, aircraft_by_name()
│   ├── airport.rs          # Airport struct, include!() generated DB, filtering
│   ├── flight_plan.rs      # FlightPlan struct, block time calculation
│   ├── error.rs            # Error enum (thiserror)
│   ├── selection.rs        # airport selection algorithm, retry logic
│   └── bin/
│       └── main.rs         # CLI binary (clap)
├── data/                   # committed CSV fallback for offline builds
└── tests/
    └── integration.rs      # end-to-end tests
```

**Changes from spec:** Split `error.rs` and `selection.rs` out of `lib.rs` and `flight_plan.rs` respectively for clearer boundaries. The spec's module layout is preserved conceptually.

---

### Task 1: Project Scaffold & Cargo.toml

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs` (empty placeholder)
- Create: `src/bin/main.rs` (minimal placeholder)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "random-flight"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
clap = { version = "4", features = ["derive"] }
humantime = "2"
rand = "0.9"
thiserror = "2"

[build-dependencies]
csv = "1"
serde = { version = "1", features = ["derive"] }
ureq = "3"
```

Note: `time` crate is NOT needed — `std::time::Duration` is sufficient for our use case (we only do arithmetic on durations, no calendar/clock operations). `serde` and `csv` are build-dependencies only (used in `build.rs` to parse CSVs). `ureq` replaces `reqwest` for build-script HTTP — it's synchronous, has no tokio dependency, and compiles fast.

- [ ] **Step 2: Create placeholder src/lib.rs**

```rust
pub mod geo;
```

- [ ] **Step 3: Create placeholder src/geo.rs**

```rust
// Haversine distance calculation
```

- [ ] **Step 4: Create placeholder src/bin/main.rs**

```rust
fn main() {
    println!("random-flight");
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully with warnings about unused imports (that's fine).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/
git commit -m "chore: scaffold project with Cargo.toml and placeholders"
```

---

### Task 2: Geo Module — Haversine Distance

**Files:**
- Create: `src/geo.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing tests in geo.rs**

```rust
/// Calculates great-circle distance in nautical miles between two points.
pub fn haversine_distance_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_distance_jfk_to_lhr() {
        // JFK (40.6413, -73.7781) to LHR (51.4700, -0.4543)
        // Known great-circle distance: ~2999 nm (source: gcmap.com: 2999 nm)
        let d = haversine_distance_nm(40.6413, -73.7781, 51.4700, -0.4543);
        assert!((d - 2999.0).abs() < 10.0, "JFK-LHR distance was {d}, expected ~2999 nm");
    }

    #[test]
    fn known_distance_sfo_to_nrt() {
        // SFO (37.6213, -122.3790) to NRT (35.7647, 140.3864)
        // Known great-circle distance: ~4476 nm
        let d = haversine_distance_nm(37.6213, -122.3790, 35.7647, 140.3864);
        assert!((d - 4476.0).abs() < 10.0, "SFO-NRT distance was {d}, expected ~4476 nm");
    }

    #[test]
    fn zero_distance_same_point() {
        let d = haversine_distance_nm(51.4700, -0.4543, 51.4700, -0.4543);
        assert!(d.abs() < 0.01, "Same point distance should be ~0, was {d}");
    }

    #[test]
    fn short_distance_eddf_to_eddm() {
        // Frankfurt (50.0379, 8.5622) to Munich (48.3537, 11.7750)
        // Known: ~152 nm
        let d = haversine_distance_nm(50.0379, 8.5622, 48.3537, 11.7750);
        assert!((d - 152.0).abs() < 5.0, "EDDF-EDDM distance was {d}, expected ~152 nm");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib geo`
Expected: FAIL — `todo!()` panics.

- [ ] **Step 3: Implement haversine_distance_nm**

Replace the `todo!()` body with:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib geo`
Expected: All 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/geo.rs src/lib.rs
git commit -m "feat: add haversine great-circle distance calculation"
```

---

### Task 3: Error Types

**Files:**
- Create: `src/error.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no airports match the aircraft's runway requirements")]
    NoValidAirports,

    #[error("no candidate arrival airports found within distance band")]
    NoCandidateArrivals,

    #[error("exhausted {attempts} retries without finding a valid pair")]
    RetriesExhausted { attempts: u32 },

    #[error("unknown airport ICAO code: {icao}")]
    UnknownAirport { icao: String },

    #[error("flight distance {distance_nm:.0} nm exceeds aircraft range of {range_nm} nm")]
    RangeExceeded { distance_nm: f64, range_nm: u32 },

    #[error("runway at {airport_icao} is {available_ft} ft, aircraft requires {required_ft} ft")]
    RunwayTooShort {
        airport_icao: String,
        required_ft: u32,
        available_ft: u32,
    },
}
```

- [ ] **Step 2: Add to lib.rs**

```rust
pub mod error;
pub mod geo;

pub use error::Error;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles.

- [ ] **Step 4: Commit**

```bash
git add src/error.rs src/lib.rs
git commit -m "feat: add error types"
```

---

### Task 4: Aircraft Module

**Files:**
- Create: `src/aircraft.rs`
- Modify: `src/lib.rs`

**Spec deviation:** `Aircraft.name` uses `&'static str` instead of `String` (spec). This enables static presets without lazy init. Users building custom aircraft use string literals or `Box::leak` for dynamic names. The CLI handles this with `"Custom"` literal.

- [ ] **Step 1: Write Aircraft struct with presets and tests**

The struct, presets, lookup functions, and tests are all in one step since the tests depend on presets existing.

```rust
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
```

Add tests after the presets in the same file:

```rust
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
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test --lib aircraft`
Expected: All 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/aircraft.rs src/lib.rs
git commit -m "feat: add aircraft struct with built-in presets"
```

---

### Task 5: Build Script & Airport Database

**Files:**
- Create: `build.rs`
- Create: `src/airport.rs`
- Create: `data/.gitkeep` (placeholder until data downloaded)
- Modify: `src/lib.rs`

This is the most complex task. `build.rs` downloads the OurAirports CSVs, joins airports with their longest hard-surface runway, and generates a Rust source file with a static array.

- [ ] **Step 1: Create build.rs**

```rust
use csv::ReaderBuilder;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const AIRPORTS_URL: &str = "https://davidmegginson.github.io/ourairports-data/airports.csv";
const RUNWAYS_URL: &str = "https://davidmegginson.github.io/ourairports-data/runways.csv";

#[derive(Debug, Deserialize)]
struct RawAirport {
    ident: String,
    #[serde(rename = "type")]
    airport_type: String,
    name: String,
    latitude_deg: f64,
    longitude_deg: f64,
    elevation_ft: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRunway {
    airport_ident: String,
    length_ft: Option<String>,
    surface: Option<String>,
    closed: Option<u8>,
}

fn download_or_fallback(url: &str, fallback_path: &Path, dest_path: &Path) {
    if let Ok(body) = try_download(url) {
        fs::write(dest_path, &body).expect("failed to write downloaded CSV");
        // Also update fallback
        fs::create_dir_all(fallback_path.parent().unwrap()).ok();
        fs::write(fallback_path, &body).ok();
    } else {
        eprintln!("cargo:warning=Failed to download {url}, using fallback");
        if fallback_path.exists() {
            fs::copy(fallback_path, dest_path).expect("fallback copy failed");
        } else {
            panic!("No fallback data at {} and download failed", fallback_path.display());
        }
    }
}

fn try_download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;
    let mut body = Vec::new();
    response.into_body().read_to_end(&mut body)?;
    Ok(body)
}

fn is_hard_surface(surface: &str) -> bool {
    let s = surface.to_uppercase();
    s.starts_with("ASP") || s.starts_with("CON") || s.starts_with("BIT")
        || s.starts_with("PEM") || s == "ASPHALT" || s == "CONCRETE"
        || s == "BITUMINOUS" || s.starts_with("ASPH") || s.starts_with("CONC")
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let data_dir = manifest_dir.join("data");

    let airports_tmp = out_dir.join("airports.csv");
    let runways_tmp = out_dir.join("runways.csv");

    download_or_fallback(AIRPORTS_URL, &data_dir.join("airports.csv"), &airports_tmp);
    download_or_fallback(RUNWAYS_URL, &data_dir.join("runways.csv"), &runways_tmp);

    // Parse runways: map airport_ident -> longest hard-surface non-closed runway length
    let mut best_runway: HashMap<String, u32> = HashMap::new();
    {
        let mut rdr = ReaderBuilder::new().from_path(&runways_tmp).expect("open runways.csv");
        for result in rdr.deserialize::<RawRunway>() {
            let rwy = match result {
                Ok(r) => r,
                Err(_) => continue,
            };
            if rwy.closed == Some(1) {
                continue;
            }
            let surface = match &rwy.surface {
                Some(s) if is_hard_surface(s) => true,
                _ => false,
            };
            if !surface {
                continue;
            }
            let length: u32 = match &rwy.length_ft {
                Some(s) => match s.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                },
                None => continue,
            };
            let entry = best_runway.entry(rwy.airport_ident.clone()).or_insert(0);
            if length > *entry {
                *entry = length;
            }
        }
    }

    // Parse airports, filter, and generate code
    let mut airports = Vec::new();
    {
        let mut rdr = ReaderBuilder::new().from_path(&airports_tmp).expect("open airports.csv");
        for result in rdr.deserialize::<RawAirport>() {
            let apt = match result {
                Ok(a) => a,
                Err(_) => continue,
            };
            // Skip non-airports
            match apt.airport_type.as_str() {
                "large_airport" | "medium_airport" | "small_airport" => {}
                _ => continue,
            }
            // Must have ICAO-style ident (4 letters)
            if apt.ident.len() != 4 || !apt.ident.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }
            // Must have a hard-surface runway
            let runway_len = match best_runway.get(&apt.ident) {
                Some(&len) => len,
                None => continue,
            };
            let elevation: i32 = apt
                .elevation_ft
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            airports.push((
                apt.ident.clone(),
                apt.name.replace('\\', "\\\\").replace('"', "\\\""),
                apt.latitude_deg,
                apt.longitude_deg,
                elevation,
                runway_len,
            ));
        }
    }

    // Generate Rust source
    let gen_path = out_dir.join("airport_db.rs");
    let mut f = fs::File::create(&gen_path).expect("create airport_db.rs");

    writeln!(f, "static AIRPORTS: &[Airport] = &[").unwrap();
    for (icao, name, lat, lon, elev, rwy) in &airports {
        writeln!(
            f,
            "    Airport {{ icao: \"{icao}\", name: \"{name}\", latitude: {lat}_f64, longitude: {lon}_f64, elevation_ft: {elev}, runway_length_ft: {rwy} }},"
        )
        .unwrap();
    }
    writeln!(f, "];").unwrap();

    println!("cargo:rerun-if-changed=data/airports.csv");
    println!("cargo:rerun-if-changed=data/runways.csv");
    println!("cargo:warning=Generated airport DB with {} airports", airports.len());
}
```

- [ ] **Step 2: Create src/airport.rs**

```rust
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
    let icao_upper = icao.to_uppercase();
    AIRPORTS.iter().find(|a| a.icao == icao_upper)
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
```

- [ ] **Step 3: Create data directory**

```bash
mkdir -p data
touch data/.gitkeep
```

- [ ] **Step 4: Update src/lib.rs**

```rust
pub mod aircraft;
pub mod airport;
pub mod error;
pub mod geo;

pub use aircraft::{Aircraft, aircraft_by_name, built_in_aircraft};
pub use airport::Airport;
pub use error::Error;
```

- [ ] **Step 5: Build and run tests**

Run: `cargo test --lib airport`
Expected: Build downloads CSVs (first time), generates airport_db.rs, all 5 tests PASS. Build might take a moment due to CSV download.

- [ ] **Step 6: Commit data fallback and source**

```bash
git add build.rs src/airport.rs src/lib.rs data/
git commit -m "feat: add build script and airport database with OurAirports data"
```

---

### Task 6: Flight Plan Calculation

**Files:**
- Create: `src/flight_plan.rs`
- Modify: `src/lib.rs`

**Spec deviation:** `FlightPlan.departure/arrival` use `&'static Airport` instead of owned `Airport` (spec). All airports come from the static generated DB, so references avoid cloning. If future versions need dynamic airports, this would need to change to owned or generic lifetime.

- [ ] **Step 1: Write FlightPlan struct and calculation tests**

```rust
use std::time::Duration;

use crate::aircraft::Aircraft;
use crate::airport::Airport;
use crate::geo::haversine_distance_nm;

#[derive(Debug, Clone)]
pub struct FlightPlan {
    pub departure: &'static Airport,
    pub arrival: &'static Airport,
    pub aircraft: Aircraft,
    pub distance_nm: f64,
    pub block_time: Duration,
    pub taxi_time: Duration,
    pub cruise_altitude_ft: u32,
    pub climb_time: Duration,
    pub climb_distance_nm: f64,
    pub descent_time: Duration,
    pub descent_distance_nm: f64,
    pub cruise_time: Duration,
    pub cruise_distance_nm: f64,
}

pub fn calculate_flight_plan(
    departure: &'static Airport,
    arrival: &'static Airport,
    aircraft: &Aircraft,
    taxi_time: Duration,
) -> FlightPlan {
    let distance_nm = haversine_distance_nm(
        departure.latitude, departure.longitude,
        arrival.latitude, arrival.longitude,
    );

    let cruise_speed = aircraft.cruise_speed_kts as f64;
    let climb_speed = cruise_speed * aircraft.climb_speed_factor as f64;
    let descent_speed = cruise_speed * aircraft.descent_speed_factor as f64;

    let mut cruise_alt = aircraft.cruise_altitude_ft;
    let min_alt = (departure.elevation_ft.max(arrival.elevation_ft) + 1000) as u32;

    loop {
        let climb_ft = cruise_alt.saturating_sub(departure.elevation_ft.max(0) as u32) as f64;
        let descent_ft = cruise_alt.saturating_sub(arrival.elevation_ft.max(0) as u32) as f64;

        let climb_time_hrs = climb_ft / aircraft.climb_rate_fpm as f64 / 60.0;
        let descent_time_hrs = descent_ft / aircraft.descent_rate_fpm as f64 / 60.0;

        let climb_dist = climb_speed * climb_time_hrs;
        let descent_dist = descent_speed * descent_time_hrs;

        if climb_dist + descent_dist < distance_nm || cruise_alt <= min_alt {
            let cruise_dist = (distance_nm - climb_dist - descent_dist).max(0.0);
            let cruise_time_hrs = cruise_dist / cruise_speed;

            let to_duration = |hrs: f64| Duration::from_secs_f64(hrs * 3600.0);

            let climb_time = to_duration(climb_time_hrs);
            let descent_time = to_duration(descent_time_hrs);
            let cruise_time = to_duration(cruise_time_hrs);
            let block_time = climb_time + descent_time + cruise_time + taxi_time;

            return FlightPlan {
                departure,
                arrival,
                aircraft: aircraft.clone(),
                distance_nm,
                block_time,
                taxi_time,
                cruise_altitude_ft: cruise_alt,
                climb_time,
                climb_distance_nm: climb_dist,
                descent_time,
                descent_distance_nm: descent_dist,
                cruise_time,
                cruise_distance_nm: cruise_dist,
            };
        }

        // Reduce altitude and retry
        cruise_alt = cruise_alt.saturating_sub(1000).max(min_alt);
    }
}

/// Estimate the cruise-only distance for a target block time.
/// Used by the selection algorithm to narrow the airport search band.
pub fn estimate_distance_for_block_time(
    aircraft: &Aircraft,
    target_block_time: Duration,
    taxi_time: Duration,
) -> f64 {
    let flight_time_hrs = target_block_time
        .saturating_sub(taxi_time)
        .as_secs_f64() / 3600.0;
    // Rough estimate: assume mostly cruise with some time lost to climb/descent
    // Use 90% of cruise speed as effective speed to account for climb/descent phases
    let effective_speed = aircraft.cruise_speed_kts as f64 * 0.90;
    effective_speed * flight_time_hrs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aircraft::aircraft_by_name;
    use crate::airport::find_by_icao;

    fn taxi() -> Duration {
        Duration::from_secs(600) // 10 min
    }

    #[test]
    fn basic_flight_plan_computes() {
        let dep = find_by_icao("KJFK").expect("KJFK");
        let arr = find_by_icao("KLAX").expect("KLAX");
        let ac = aircraft_by_name("B738").expect("B738");

        let fp = calculate_flight_plan(dep, arr, ac, taxi());

        // JFK-LAX is ~2145 nm, B738 at 460 kts ~ 4.7h flight + taxi
        assert!(fp.distance_nm > 2000.0 && fp.distance_nm < 2300.0,
            "distance was {} nm", fp.distance_nm);
        assert!(fp.block_time.as_secs() > 4 * 3600, "block time too short");
        assert!(fp.block_time.as_secs() < 6 * 3600, "block time too long: {}s", fp.block_time.as_secs());
        assert!(fp.cruise_distance_nm > 0.0);
        assert!(fp.climb_time.as_secs() > 0);
        assert!(fp.descent_time.as_secs() > 0);
    }

    #[test]
    fn short_flight_reduces_altitude() {
        let dep = find_by_icao("EDDF").expect("EDDF");
        let arr = find_by_icao("EDDM").expect("EDDM");
        let ac = aircraft_by_name("B738").expect("B738");

        let fp = calculate_flight_plan(dep, arr, ac, taxi());

        // ~152 nm, jet can't reach FL360 — altitude should be reduced
        assert!(fp.cruise_altitude_ft < 36000,
            "cruise alt should be reduced for short flight, was {}", fp.cruise_altitude_ft);
        assert!(fp.cruise_distance_nm >= 0.0);
    }

    #[test]
    fn block_time_equals_sum_of_phases() {
        let dep = find_by_icao("KJFK").expect("KJFK");
        let arr = find_by_icao("EGLL").expect("EGLL");
        let ac = aircraft_by_name("B738").expect("B738");

        let fp = calculate_flight_plan(dep, arr, ac, taxi());

        let sum = fp.climb_time + fp.cruise_time + fp.descent_time + fp.taxi_time;
        let diff = if fp.block_time > sum {
            fp.block_time - sum
        } else {
            sum - fp.block_time
        };
        assert!(diff.as_millis() < 10, "block_time should equal sum of phases");
    }

    #[test]
    fn c172_short_hop() {
        let dep = find_by_icao("EDDF").expect("EDDF");
        let arr = find_by_icao("EDDM").expect("EDDM");
        let ac = aircraft_by_name("C172").expect("C172");

        let fp = calculate_flight_plan(dep, arr, ac, taxi());

        // ~152 nm at 122 kts ~ 1.25h + climb/descent + taxi ~ 1.5-2h
        assert!(fp.block_time.as_secs() > 3600, "too short");
        assert!(fp.block_time.as_secs() < 3 * 3600, "too long");
    }

    #[test]
    fn estimate_distance_reasonable() {
        let ac = aircraft_by_name("B738").expect("B738");
        let target = Duration::from_secs(2 * 3600); // 2 hours

        let est = estimate_distance_for_block_time(ac, target, taxi());

        // B738 at ~414 kts effective * 1.83h ≈ 760 nm
        assert!(est > 600.0 && est < 1000.0, "estimate was {est} nm");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib flight_plan`
Expected: FAIL — module doesn't exist yet in lib.rs (or `todo!()` if you wrote stubs).

- [ ] **Step 3: Update lib.rs to include flight_plan module**

Add `pub mod flight_plan;` to lib.rs and the re-export:
```rust
pub use flight_plan::{FlightPlan, calculate_flight_plan, estimate_distance_for_block_time};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib flight_plan`
Expected: All 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/flight_plan.rs src/lib.rs
git commit -m "feat: add flight plan calculation with climb/cruise/descent model"
```

---

### Task 7: Selection Algorithm

**Files:**
- Create: `src/selection.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write selection logic with tests**

```rust
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

        let diff = if fp.block_time > target_block_time {
            fp.block_time - target_block_time
        } else {
            target_block_time - fp.block_time
        };

        if diff <= opts.tolerance {
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib selection`
Expected: FAIL — module doesn't exist in lib.rs yet.

- [ ] **Step 3: Update lib.rs**

Add `pub mod selection;` and re-exports:
```rust
pub use selection::{FlightPlanOptions, generate_flight_plan, generate_flight_plan_with_rng};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib selection`
Expected: All 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/selection.rs src/lib.rs
git commit -m "feat: add airport selection algorithm with retry logic"
```

---

### Task 8: CLI Binary

**Files:**
- Modify: `src/bin/main.rs`

- [ ] **Step 1: Implement CLI**

```rust
use std::process;
use std::time::Duration;

use clap::Parser;

use random_flight::{
    Aircraft, FlightPlanOptions, aircraft_by_name, built_in_aircraft,
    generate_flight_plan,
};

#[derive(Parser)]
#[command(name = "random-flight", about = "Generate random flight plans for flight simulators")]
struct Cli {
    /// Aircraft preset name (e.g. C172, B738, A320) or "custom"
    #[arg(long)]
    aircraft: String,

    /// Target block time (e.g. 2h, 2h30m, 90m)
    #[arg(long)]
    time: String,

    /// Tolerance around target time (default: 15m)
    #[arg(long, default_value = "15m")]
    tolerance: String,

    /// Pin departure airport (ICAO code)
    #[arg(long)]
    departure: Option<String>,

    /// Pin arrival airport (ICAO code)
    #[arg(long)]
    arrival: Option<String>,

    /// Custom aircraft: cruise speed in knots
    #[arg(long)]
    cruise_speed: Option<u16>,

    /// Custom aircraft: cruise altitude in feet
    #[arg(long)]
    cruise_alt: Option<u32>,

    /// Custom aircraft: climb rate in ft/min
    #[arg(long)]
    climb_rate: Option<u16>,

    /// Custom aircraft: descent rate in ft/min
    #[arg(long)]
    descent_rate: Option<u16>,

    /// Custom aircraft: range in nautical miles
    #[arg(long)]
    range: Option<u32>,

    /// Custom aircraft: minimum runway length in feet
    #[arg(long)]
    min_runway: Option<u32>,

    /// List available aircraft presets
    #[arg(long)]
    list_aircraft: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.list_aircraft {
        println!("Available aircraft presets:");
        for a in built_in_aircraft() {
            println!("  {:<6} {} kts, FL{:03}, range {} nm, min rwy {} ft",
                a.name, a.cruise_speed_kts, a.cruise_altitude_ft / 100,
                a.range_nm, a.min_runway_length_ft);
        }
        return;
    }

    let aircraft = if cli.aircraft.eq_ignore_ascii_case("custom") {
        let missing = |field: &str| -> ! {
            eprintln!("Error: --{field} is required for custom aircraft");
            process::exit(1);
        };
        Aircraft {
            name: "Custom",
            cruise_speed_kts: cli.cruise_speed.unwrap_or_else(|| missing("cruise-speed")),
            cruise_altitude_ft: cli.cruise_alt.unwrap_or_else(|| missing("cruise-alt")),
            climb_rate_fpm: cli.climb_rate.unwrap_or_else(|| missing("climb-rate")),
            descent_rate_fpm: cli.descent_rate.unwrap_or_else(|| missing("descent-rate")),
            climb_speed_factor: 0.75,
            descent_speed_factor: 0.65,
            range_nm: cli.range.unwrap_or_else(|| missing("range")),
            min_runway_length_ft: cli.min_runway.unwrap_or_else(|| missing("min-runway")),
        }
    } else {
        match aircraft_by_name(&cli.aircraft) {
            Some(a) => a.clone(),
            None => {
                eprintln!("Error: unknown aircraft '{}'. Use --list-aircraft to see presets.", cli.aircraft);
                process::exit(1);
            }
        }
    };

    let target = parse_duration(&cli.time);
    let tolerance = parse_duration(&cli.tolerance);

    let opts = FlightPlanOptions {
        tolerance,
        departure_icao: cli.departure,
        arrival_icao: cli.arrival,
        ..Default::default()
    };

    match generate_flight_plan(&aircraft, target, Some(opts)) {
        Ok(fp) => {
            println!("Flight Plan");
            println!("===========");
            println!("Aircraft:    {}", fp.aircraft.name);
            println!("Departure:   {} ({})", fp.departure.icao, fp.departure.name);
            println!("Arrival:     {} ({})", fp.arrival.icao, fp.arrival.name);
            println!("Distance:    {:.0} nm", fp.distance_nm);
            println!("Block Time:  {}", format_duration(fp.block_time));
            println!();
            println!("Cruise Alt:  {} ft", fp.cruise_altitude_ft);
            println!("Climb:       {} ({:.0} nm)", format_duration(fp.climb_time), fp.climb_distance_nm);
            println!("Cruise:      {} ({:.0} nm)", format_duration(fp.cruise_time), fp.cruise_distance_nm);
            println!("Descent:     {} ({:.0} nm)", format_duration(fp.descent_time), fp.descent_distance_nm);
            println!("Taxi:        {}", format_duration(fp.taxi_time));
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn parse_duration(s: &str) -> Duration {
    humantime::parse_duration(s).unwrap_or_else(|e| {
        eprintln!("Error: invalid duration '{s}': {e}");
        process::exit(1);
    })
}

fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    if h > 0 {
        format!("{h}h {m:02}m")
    } else {
        format!("{m}m")
    }
}
```

- [ ] **Step 2: Verify it compiles and runs**

Run: `cargo run -- --list-aircraft`
Expected: Prints the aircraft preset table.

Run: `cargo run -- --aircraft B738 --time 2h`
Expected: Prints a random flight plan with ~2h block time.

Run: `cargo run -- --aircraft B738 --time 2h --departure EDDF`
Expected: Prints a flight departing from Frankfurt.

- [ ] **Step 3: Commit**

```bash
git add src/bin/main.rs
git commit -m "feat: add CLI binary with clap"
```

---

### Task 9: Integration Tests

**Files:**
- Create: `tests/integration.rs`

- [ ] **Step 1: Write integration tests**

```rust
use std::time::Duration;

use rand::SeedableRng;
use rand::rngs::SmallRng;

use random_flight::{
    FlightPlanOptions, aircraft_by_name, generate_flight_plan_with_rng,
};

#[test]
fn c172_one_hour_flight() {
    let ac = aircraft_by_name("C172").unwrap();
    let target = Duration::from_secs(3600);
    let opts = FlightPlanOptions {
        tolerance: Duration::from_secs(15 * 60),
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(123);
    let fp = generate_flight_plan_with_rng(ac, target, Some(opts), &mut rng).unwrap();

    let diff = fp.block_time.as_secs().abs_diff(target.as_secs());
    assert!(diff <= 15 * 60, "block time off by {diff}s");
}

#[test]
fn b738_four_hour_flight() {
    let ac = aircraft_by_name("B738").unwrap();
    let target = Duration::from_secs(4 * 3600);
    let opts = FlightPlanOptions {
        tolerance: Duration::from_secs(15 * 60),
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(456);
    let fp = generate_flight_plan_with_rng(ac, target, Some(opts), &mut rng).unwrap();

    let diff = fp.block_time.as_secs().abs_diff(target.as_secs());
    assert!(diff <= 15 * 60, "block time off by {diff}s");
}

#[test]
fn pinned_route_jfk_lax() {
    let ac = aircraft_by_name("B738").unwrap();
    let opts = FlightPlanOptions {
        departure_icao: Some("KJFK".into()),
        arrival_icao: Some("KLAX".into()),
        ..Default::default()
    };
    let mut rng = SmallRng::seed_from_u64(789);
    let fp = generate_flight_plan_with_rng(ac, Duration::from_secs(3600), Some(opts), &mut rng).unwrap();

    assert_eq!(fp.departure.icao, "KJFK");
    assert_eq!(fp.arrival.icao, "KLAX");
    assert!(fp.distance_nm > 2000.0);
}

#[test]
fn deterministic_with_same_seed() {
    let ac = aircraft_by_name("A320").unwrap();
    let target = Duration::from_secs(3 * 3600);
    let opts1 = FlightPlanOptions::default();
    let opts2 = FlightPlanOptions::default();

    let mut rng1 = SmallRng::seed_from_u64(42);
    let mut rng2 = SmallRng::seed_from_u64(42);

    let fp1 = generate_flight_plan_with_rng(ac, target, Some(opts1), &mut rng1).unwrap();
    let fp2 = generate_flight_plan_with_rng(ac, target, Some(opts2), &mut rng2).unwrap();

    assert_eq!(fp1.departure.icao, fp2.departure.icao);
    assert_eq!(fp1.arrival.icao, fp2.arrival.icao);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test --test integration`
Expected: All 4 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for flight plan generation"
```

---

### Task 10: Final Verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All unit and integration tests PASS.

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 3: Test CLI end-to-end**

Run: `cargo run -- --aircraft C172 --time 1h30m`
Run: `cargo run -- --aircraft A320 --time 3h --departure LFPG`
Run: `cargo run -- --aircraft B738 --time 5h --departure KJFK --arrival EGLL`
Expected: All produce reasonable output.

- [ ] **Step 4: Fix any clippy or test issues found**

- [ ] **Step 5: Final commit if any fixes were needed**

```bash
git add -A
git commit -m "chore: final cleanup and lint fixes"
```
