# Random Flight Plan Generator — Design Spec

## Purpose

A Rust crate that generates random flight plans for flight simulator players. Given an aircraft and a target block time, it selects a pair of airports that produces a flight matching the requested duration. The goal is helping sim players find flights that fit their available time.

All calculations assume zero-wind, great-circle routing. Block times are estimates suitable for flight sim planning, not real-world OFP accuracy.

## Data Model

### Airport

```rust
struct Airport {
    icao: String,
    name: String,
    latitude: f64,
    longitude: f64,
    elevation_ft: i32,
    runway_length_ft: u32, // longest hard-surface, non-closed runway
}
```

Source: OurAirports CSV dataset, all ICAO airports. Downloaded and code-generated at build time via `build.rs`. Runway data filtered to hard-surface (asphalt/concrete), non-closed runways only. Helipads excluded.

### Aircraft

```rust
struct Aircraft {
    name: String,
    cruise_speed_kts: u16,
    cruise_altitude_ft: u32,
    climb_rate_fpm: u16,
    descent_rate_fpm: u16,
    climb_speed_factor: f32,   // fraction of cruise speed during climb (default 0.75)
    descent_speed_factor: f32, // fraction of cruise speed during descent (default 0.65)
    range_nm: u32,
    min_runway_length_ft: u32,
}
```

The crate ships built-in presets (C172, B738, A320, etc.) and accepts custom `Aircraft` values. Speed factors are per-aircraft to avoid unrealistic approximations across different aircraft categories.

### FlightPlan

```rust
struct FlightPlan {
    departure: Airport,
    arrival: Airport,
    aircraft: Aircraft,
    distance_nm: f64,
    block_time: Duration,
    taxi_time: Duration,
    cruise_altitude_ft: u32,
    climb_time: Duration,
    climb_distance_nm: f64,
    descent_time: Duration,
    descent_distance_nm: f64,
    cruise_time: Duration,
    cruise_distance_nm: f64,
}
```

`block_time = climb_time + cruise_time + descent_time + taxi_time`. All phase durations are explicit.

## Block Time Calculation

Direct flight, simplified model (can be made more realistic later):

1. **Great-circle distance** between departure and arrival using Haversine formula.
2. **Climb phase:** Altitude gain = cruise_altitude - departure_elevation. Time = gain / climb_rate. Distance at climb_speed_factor * cruise_speed.
3. **Descent phase:** Altitude loss = cruise_altitude - arrival_elevation. Time = loss / descent_rate. Distance at descent_speed_factor * cruise_speed.
4. **Cruise phase:** Remaining distance at cruise speed.
5. **Block time** = climb + cruise + descent + taxi buffer (configurable, default ~10 min total).

**Short flight altitude adjustment:** If climb_distance + descent_distance >= total distance, the cruise altitude is iteratively reduced until a valid cruise segment exists. The minimum cruise altitude is the higher of the two airport elevations + 1000 ft.

## Airport Selection Algorithm

Given target block time, tolerance, and aircraft:

1. **Pre-filter airports** — exclude airports with runways shorter than aircraft's minimum.
2. **Estimate target distance** — back-calculate approximate distance from target block time and aircraft performance.
3. **Pick random departure** from filtered set.
4. **Find candidate arrivals** — airports within a distance band (derived from tolerance) around target distance, also passing runway filter and aircraft range check.
5. **Pick random arrival** from candidates.
6. **Compute exact block time** — verify it falls within tolerance.
7. **Retry** on failure (bounded, then return error).

**Both airports pinned:** If both `departure_icao` and `arrival_icao` are set, skip random selection entirely — compute the flight plan for the given pair and return it (or error if infeasible for the aircraft).

## Error Handling

```rust
pub enum Error {
    /// No airports match the aircraft's runway requirements
    NoValidAirports,
    /// No candidate arrival airports found within distance band
    NoCandidateArrivals,
    /// Exhausted retries without finding a valid pair
    RetriesExhausted { attempts: u32 },
    /// Specified ICAO code not found in database
    UnknownAirport { icao: String },
    /// Flight exceeds aircraft range
    RangeExceeded { distance_nm: f64, range_nm: u32 },
    /// Aircraft runway requirement exceeds airport runway length
    RunwayTooShort { airport_icao: String, required_ft: u32, available_ft: u32 },
}
```

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
├── data/                 # cached airport CSV (downloaded by build.rs, committed as fallback)
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

pub fn generate_flight_plan_with_rng(
    aircraft: &Aircraft,
    target_block_time: Duration,
    options: Option<FlightPlanOptions>,
    rng: &mut impl Rng,
) -> Result<FlightPlan, Error>;

pub struct FlightPlanOptions {
    pub tolerance: Duration,            // default 15 min
    pub taxi_time: Duration,            // default 10 min
    pub max_retries: u32,               // default 100
    pub departure_icao: Option<String>, // pin departure
    pub arrival_icao: Option<String>,   // pin arrival
}

pub fn built_in_aircraft() -> &'static [Aircraft];
pub fn aircraft_by_name(name: &str) -> Option<&'static Aircraft>;
```

## CLI

```
random-flight --aircraft C172 --time 2h
random-flight --aircraft C172 --time 2h --tolerance 10m
random-flight --aircraft C172 --time 2h --departure EDDF
random-flight --aircraft custom --time 3h \
    --cruise-speed 250 --cruise-alt 25000 --climb-rate 1500 \
    --descent-rate 1000 --range 1200 --min-runway 5000
```

Duration parsing for `--time` and `--tolerance` uses `humantime` for human-friendly input (e.g. `2h`, `2h30m`, `90m`).

## Dependencies

- `time` — duration handling and formatting
- `clap` — CLI argument parsing
- `rand` — random selection
- `serde` + `csv` — build script CSV parsing
- `thiserror` — error types
- `humantime` — CLI duration parsing

## Build Script

`build.rs` downloads the OurAirports `airports.csv` and `runways.csv` to `data/`, parses them, and generates a Rust source file containing a static array of `Airport` structs. The generated file is included via `include!()` in `airport.rs`.

A committed snapshot of the data files in `data/` serves as fallback for offline builds. The build script attempts to download fresh data but falls back to the committed snapshot if the download fails.

## Design Decisions

- **`time` over `chrono`**: We only need durations, not datetimes. `time` is lighter.
- **Build-time codegen over runtime parsing**: Self-contained binary, no startup cost, offline-capable.
- **Simplified flight model**: Intentionally basic (climb/cruise/descent). Zero-wind, great-circle. Can be made more realistic later.
- **Configurable tolerance with sensible default**: 15 min default keeps it practical without being too loose.
- **Per-aircraft speed factors**: Avoids hardcoded climb/descent speed ratios that don't work across GA and jets.
- **Committed data fallback**: Ensures the crate builds offline after initial clone.

## Out of Scope (v1)

- Region/geographic filtering (e.g. "only European airports") — planned for future version
- Wind modeling
- SID/STAR routing
- Fuel calculations
- Multi-leg flights
