# Taxi Time Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the flat 10-minute taxi time with per-airport taxi times based on OurAirports' airport size classification, producing block times closer to SimBrief.

**Architecture:** Add `AirportSize` enum to airport data, carry it through the build pipeline, and use it in `flight_plan.rs` to compute taxi out/in per departure/arrival airport. The selection algorithm uses a medium-airport estimate as the search heuristic.

**Tech Stack:** Rust, `build.rs` code generation, OurAirports CSV data

**Spec:** `docs/superpowers/specs/2026-03-24-taxi-time-model-design.md`

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `src/airport.rs` | Modify | Add `AirportSize` enum, add `size` field to `Airport` |
| `build.rs` | Modify | Carry `airport_type` into generated DB as `AirportSize` variant |
| `src/flight_plan.rs` | Modify | Add `taxi_times()`, remove `taxi_time` param, split into `taxi_out`/`taxi_in` |
| `src/selection.rs` | Modify | Remove `taxi_time` from options/calls, hardcode medium estimate |
| `src/lib.rs` | Modify | Add `AirportSize` re-export |
| `src/main.rs` | Modify | Display taxi out/in separately |
| `tests/integration.rs` | Modify | Update CLI output assertion for new taxi format |

---

### Task 1: Add AirportSize enum and update Airport struct

**Files:**
- Modify: `src/airport.rs:1-9`

- [ ] **Step 1: Write test for AirportSize on a known airport**

Add to `src/airport.rs` tests module:

```rust
use crate::airport::AirportSize;

#[test]
fn kjfk_is_large_airport() {
    let apt = find_by_icao("KJFK").expect("KJFK should exist");
    assert_eq!(apt.size, AirportSize::Large);
}

#[test]
fn airport_sizes_vary() {
    let all = all_airports();
    let large = all.iter().filter(|a| a.size == AirportSize::Large).count();
    let medium = all.iter().filter(|a| a.size == AirportSize::Medium).count();
    let small = all.iter().filter(|a| a.size == AirportSize::Small).count();
    assert!(large > 0, "no large airports");
    assert!(medium > 0, "no medium airports");
    assert!(small > 0, "no small airports");
    assert_eq!(large + medium + small, all.len());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib airport::tests::kjfk_is_large_airport`
Expected: FAIL — `AirportSize` doesn't exist yet

- [ ] **Step 3: Add AirportSize enum and update Airport struct**

In `src/airport.rs`, add before the `Airport` struct:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirportSize {
    Large,
    Medium,
    Small,
}
```

Add to the `Airport` struct:

```rust
pub size: AirportSize,
```

- [ ] **Step 4: Update build.rs to emit airport size**

In `build.rs`, change the airports tuple push (line 128-140) to include the airport type. Change the tuple to include `apt.airport_type.clone()` as a 7th element:

```rust
airports.push((
    apt.ident.clone(),
    apt.name
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .chars()
        .filter(|c| !c.is_control() && *c != '\u{AD}')
        .collect::<String>(),
    apt.latitude_deg,
    apt.longitude_deg,
    elevation,
    runway_len,
    apt.airport_type.clone(),
));
```

Update the code generation loop (line 149-154) to destructure the 7th element and emit the `size` field:

```rust
for (icao, name, lat, lon, elev, rwy, airport_type) in &airports {
    let size_variant = match airport_type.as_str() {
        "large_airport" => "Large",
        "medium_airport" => "Medium",
        "small_airport" => "Small",
        _ => unreachable!(),
    };
    writeln!(
        f,
        "    Airport {{ icao: \"{icao}\", name: \"{name}\", latitude: {lat}_f64, longitude: {lon}_f64, elevation_ft: {elev}, runway_length_ft: {rwy}, size: AirportSize::{size_variant} }},"
    )
    .unwrap();
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --lib airport::tests`
Expected: All airport tests pass, including the two new ones

- [ ] **Step 6: Commit**

```bash
git add src/airport.rs build.rs
git commit -m "feat: add AirportSize enum to airport data model"
```

---

### Task 2: Update flight_plan.rs — taxi_times function and signature changes

**Files:**
- Modify: `src/flight_plan.rs`

- [ ] **Step 1: Write test for taxi_times varying by airport size**

Add to `src/flight_plan.rs` tests module (replace the existing `taxi()` helper):

```rust
use crate::airport::AirportSize;

#[test]
fn taxi_times_vary_by_airport_size() {
    let dep_large = find_by_icao("KJFK").expect("KJFK"); // large
    let dep_small = find_by_icao("KFNL").expect("KFNL"); // small
    let arr = find_by_icao("KLAX").expect("KLAX");

    let fp_large = calculate_flight_plan(dep_large, arr, aircraft_by_icao_type("B738").unwrap());
    let fp_small = calculate_flight_plan(dep_small, arr, aircraft_by_icao_type("B738").unwrap());

    assert!(fp_large.taxi_out > fp_small.taxi_out,
        "large dep taxi_out {:?} should exceed small dep taxi_out {:?}",
        fp_large.taxi_out, fp_small.taxi_out);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib flight_plan::tests::taxi_times_vary_by_airport_size`
Expected: FAIL — `calculate_flight_plan` still takes 4 args, `taxi_out` field doesn't exist

- [ ] **Step 3: Add taxi_times function**

Add to `src/flight_plan.rs` after the imports, before the `FlightPlan` struct:

```rust
use crate::airport::AirportSize;

fn taxi_times(departure: &Airport, arrival: &Airport) -> (Duration, Duration) {
    let taxi_out = match departure.size {
        AirportSize::Large  => Duration::from_secs(18 * 60),
        AirportSize::Medium => Duration::from_secs(10 * 60),
        AirportSize::Small  => Duration::from_secs(5 * 60),
    };
    let taxi_in = match arrival.size {
        AirportSize::Large  => Duration::from_secs(10 * 60),
        AirportSize::Medium => Duration::from_secs(6 * 60),
        AirportSize::Small  => Duration::from_secs(3 * 60),
    };
    (taxi_out, taxi_in)
}
```

- [ ] **Step 4: Update FlightPlan struct**

Replace `pub taxi_time: Duration` with:

```rust
pub taxi_out: Duration,
pub taxi_in: Duration,
```

- [ ] **Step 5: Update calculate_flight_plan signature and body**

Remove `taxi_time: Duration` parameter. New signature:

```rust
pub fn calculate_flight_plan(
    departure: &'static Airport,
    arrival: &'static Airport,
    aircraft: &Aircraft,
) -> FlightPlan {
```

Inside the function, after the `to_duration` closure, replace the `block_time` and return:

```rust
let climb_time = to_duration(climb_time_hrs);
let descent_time = to_duration(descent_time_hrs);
let cruise_time = to_duration(cruise_time_hrs);
let (taxi_out, taxi_in) = taxi_times(departure, arrival);
let block_time = climb_time + descent_time + cruise_time + taxi_out + taxi_in;

return FlightPlan {
    departure,
    arrival,
    aircraft: aircraft.clone(),
    distance_nm,
    block_time,
    taxi_out,
    taxi_in,
    cruise_altitude_ft: cruise_alt,
    climb_time,
    climb_distance_nm: climb_dist,
    descent_time,
    descent_distance_nm: descent_dist,
    cruise_time,
    cruise_distance_nm: cruise_dist,
};
```

- [ ] **Step 6: Update estimate_distance_for_block_time**

Remove `taxi_time: Duration` parameter. Hardcode medium estimate (16 min):

```rust
pub fn estimate_distance_for_block_time(
    aircraft: &Aircraft,
    target_block_time: Duration,
) -> f64 {
    // Use medium-airport taxi estimate (10 min out + 6 min in = 16 min)
    let taxi_estimate = Duration::from_secs(16 * 60);
    let flight_time_hrs = target_block_time
        .saturating_sub(taxi_estimate)
        .as_secs_f64() / 3600.0;
    let effective_speed = aircraft.cruise_speed_ktas as f64 * 0.90;
    effective_speed * flight_time_hrs
}
```

- [ ] **Step 7: Update existing tests in flight_plan.rs**

Remove the `taxi()` helper function.

Update all `calculate_flight_plan` calls to remove the taxi parameter (4 calls: `basic_flight_plan_computes`, `short_flight_reduces_altitude`, `block_time_equals_sum_of_phases`, `c172_short_hop`, `simbrief_url_contains_correct_parameters`). Example:

```rust
let fp = calculate_flight_plan(dep, arr, ac);
```

Update `block_time_equals_sum_of_phases`:

```rust
let sum = fp.climb_time + fp.cruise_time + fp.descent_time + fp.taxi_out + fp.taxi_in;
```

Update `estimate_distance_reasonable`:

```rust
let est = estimate_distance_for_block_time(ac, target);
```

Adjust the distance assertion bounds since the medium taxi estimate (16 min) vs old 10 min changes the result:

```rust
// B738 at ~414 kts effective * 1.73h ≈ 717 nm (16 min taxi subtracted from 2h)
assert!(est > 500.0 && est < 900.0, "estimate was {est} nm");
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test --lib flight_plan::tests`
Expected: All flight_plan tests pass

- [ ] **Step 9: Commit**

```bash
git add src/flight_plan.rs
git commit -m "feat: compute taxi out/in from airport size classification"
```

---

### Task 3: Update selection.rs — remove taxi_time from options

**Files:**
- Modify: `src/selection.rs`

- [ ] **Step 1: Remove taxi_time from FlightPlanOptions**

Remove `pub taxi_time: Duration` from the struct (line 13) and from the `Default` impl (line 23).

- [ ] **Step 2: Update generate_flight_plan_with_rng**

Line 50: Change `plan_for_pair(dep_icao, arr_icao, aircraft, opts.taxi_time)` to:
```rust
plan_for_pair(dep_icao, arr_icao, aircraft)
```

Line 58: Change `estimate_distance_for_block_time(aircraft, target_block_time, opts.taxi_time)` to:
```rust
estimate_distance_for_block_time(aircraft, target_block_time)
```

Line 100: Change `calculate_flight_plan(departure, arrival, aircraft, opts.taxi_time)` to:
```rust
calculate_flight_plan(departure, arrival, aircraft)
```

- [ ] **Step 3: Update plan_for_pair**

Remove `taxi_time: Duration` parameter (line 141). Update the call on line 165:

```rust
fn plan_for_pair(
    dep_icao: &str,
    arr_icao: &str,
    aircraft: &Aircraft,
) -> Result<FlightPlan, Error> {
```

```rust
Ok(calculate_flight_plan(dep, arr, aircraft))
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --lib selection::tests`
Expected: All selection tests pass (they use `..Default::default()` which no longer includes taxi_time)

- [ ] **Step 5: Commit**

```bash
git add src/selection.rs
git commit -m "refactor: remove taxi_time from FlightPlanOptions"
```

---

### Task 4: Update lib.rs re-exports

**Files:**
- Modify: `src/lib.rs:11`

- [ ] **Step 1: Add AirportSize re-export**

Change line 11 from:
```rust
pub use airport::Airport;
```
to:
```rust
pub use airport::{Airport, AirportSize};
```

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat: re-export AirportSize from public API"
```

---

### Task 5: Update main.rs display

**Files:**
- Modify: `src/main.rs:204`

- [ ] **Step 1: Update taxi display line**

Replace line 204:
```rust
println!("Taxi:        {}", format_duration(fp.taxi_time));
```

with:
```rust
println!("Taxi Out:    {}", format_duration(fp.taxi_out));
println!("Taxi In:     {}", format_duration(fp.taxi_in));
```

- [ ] **Step 2: Run CLI to verify output**

Run: `cargo run -- generate B738 2h30m`
Expected: Output shows separate `Taxi Out:` and `Taxi In:` lines with values that vary based on airport sizes

- [ ] **Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: display taxi out and taxi in separately in CLI output"
```

---

### Task 6: Update integration tests

**Files:**
- Modify: `tests/integration.rs`

- [ ] **Step 1: Update CLI output assertion**

The `cli_generate_produces_flight_plan` test (line 74) currently doesn't check for taxi specifically, so it should still pass. But add assertions for the new format:

```rust
assert!(stdout.contains("Taxi Out:"), "expected Taxi Out in output, got: {stdout}");
assert!(stdout.contains("Taxi In:"), "expected Taxi In in output, got: {stdout}");
```

- [ ] **Step 2: Run all tests**

Run: `cargo test`
Expected: All unit and integration tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy`
Expected: No warnings

- [ ] **Step 4: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add taxi out/in assertions to integration tests"
```
