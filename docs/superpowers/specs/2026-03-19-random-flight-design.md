# Random Flight Plan Generator — Design Spec

## Purpose

A Rust crate that generates random flight plans for flight simulator players. Given an aircraft and a target block time, it selects a pair of airports that produces a flight matching the requested duration. The goal is helping sim players find flights that fit their available time.

## Data Model

### Airport

```rust
struct Airport {
    icao: String,
    name: String,
    latitude: f64,
    longitude: f64,
    elevation_ft: i32,
    runway_length_ft: u32, // longest runway
}
```

Source: OurAirports CSV dataset, all ICAO airports. Downloaded and code-generated at build time via `build.rs`.

### Aircraft

```rust
struct Aircraft {
    name: String,
    cruise_speed_kts: u16,
    cruise_altitude_ft: u32,
    climb_rate_fpm: u16,
    descent_rate_fpm: u16,
    range_nm: u32,
    min_runway_length_ft: u32,
}
```

The crate ships built-in presets (C172, B738, A320, etc.) and accepts custom `Aircraft` values.

### FlightPlan

```rust
struct FlightPlan {
    departure: Airport,
    arrival: Airport,
    aircraft: Aircraft,
    distance_nm: f64,
    block_time: Duration,
    cruise_altitude_ft: u32,
    climb_time: Duration,
    climb_distance_nm: f64,
    descent_time: Duration,
    descent_distance_nm: f64,
    cruise_time: Duration,
    cruise_distance_nm: f64,
}
```

## Block Time Calculation

Direct flight, simplified model (can be made more realistic later):

1. **Great-circle distance** between departure and arrival using Haversine formula.
2. **Climb phase:** Altitude gain = cruise_altitude - departure_elevation. Time = gain / climb_rate. Distance at ~75% cruise speed.
3. **Descent phase:** Altitude loss = cruise_altitude - arrival_elevation. Time = loss / descent_rate. Distance at ~50% cruise speed.
4. **Cruise phase:** Remaining distance at cruise speed.
5. **Block time** = climb + cruise + descent + taxi buffer (configurable, default ~10 min total).

If the flight is too short for full climb to cruise altitude, cruise altitude is reduced so top-of-climb meets top-of-descent.

## Airport Selection Algorithm

Given target block time, tolerance, and aircraft:

1. **Pre-filter airports** — exclude airports with runways shorter than aircraft's minimum.
2. **Estimate target distance** — back-calculate approximate distance from target block time and aircraft performance.
3. **Pick random departure** from filtered set.
4. **Find candidate arrivals** — airports within a distance band (derived from tolerance) around target distance, also passing runway filter.
5. **Pick random arrival** from candidates.
6. **Compute exact block time** — verify it falls within tolerance.
7. **Retry** on failure (bounded, then return error).

## Crate Structure

```
random-flight/
├── Cargo.toml
├── build.rs              # downloads & codegen airport database
├── src/
│   ├── lib.rs            # public API
│   ├── airport.rs        # Airport struct, generated DB, filtering
│   ├── aircraft.rs       # Aircraft struct, built-in presets
│   ├── flight_plan.rs    # FlightPlan struct, block time calculation
│   ├── geo.rs            # Haversine distance, great-circle math
│   └── bin/
│       └── main.rs       # CLI binary (clap)
├── data/                 # cached airport CSV (gitignored, downloaded by build.rs)
└── tests/
    └── integration.rs
```

## Public API

```rust
pub fn generate_flight_plan(
    aircraft: &Aircraft,
    target_block_time: Duration,
    options: Option<FlightPlanOptions>,
) -> Result<FlightPlan, Error>;

pub struct FlightPlanOptions {
    pub tolerance: Duration,            // default 15 min
    pub taxi_time: Duration,            // default 10 min
    pub max_retries: u32,               // default 100
    pub departure_icao: Option<String>, // pin departure
    pub arrival_icao: Option<String>,   // pin arrival
}

pub fn built_in_aircraft() -> &[Aircraft];
pub fn aircraft_by_name(name: &str) -> Option<&Aircraft>;
```

## CLI

```
random-flight --aircraft C172 --time 2h
random-flight --aircraft C172 --time 2h --tolerance 10m
random-flight --aircraft C172 --time 2h --departure EDDF
```

## Dependencies

- `time` — duration handling and formatting
- `clap` — CLI argument parsing
- `rand` — random selection
- `serde` + `csv` — build script CSV parsing
- `thiserror` — error types

## Build Script

`build.rs` downloads the OurAirports `airports.csv` and `runways.csv` to `data/`, parses them, and generates a Rust source file containing a static array of `Airport` structs. The generated file is included via `include!()` in `airport.rs`.

## Design Decisions

- **`time` over `chrono`**: We only need durations, not datetimes. `time` is lighter.
- **Build-time codegen over runtime parsing**: Self-contained binary, no startup cost, offline-capable.
- **Simplified flight model**: Intentionally basic (climb/cruise/descent). Can be made more realistic later.
- **Configurable tolerance with sensible default**: 15 min default keeps it practical without being too loose.
