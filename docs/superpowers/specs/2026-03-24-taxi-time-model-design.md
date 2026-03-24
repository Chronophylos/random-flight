# Taxi Time Model Design

## Problem

The current block time calculation uses a flat 10-minute taxi time regardless of airport size. This produces block times that are significantly shorter than what SimBrief calculates for the same route. For example, a flight calculated at 2h17m block time shows up as 2h44m in SimBrief — the air time matches closely (2h16m vs 2h17m), but SimBrief adds realistic taxi-out and taxi-in times based on airport characteristics.

## Solution

Replace the flat taxi time with per-airport taxi times derived from OurAirports' airport type classification (`large_airport`, `medium_airport`, `small_airport`), which is already present in the source data but not currently stored in the compiled airport database.

## Design

### Airport size enum

Add `AirportSize` to `airport.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AirportSize {
    Large,
    Medium,
    Small,
}
```

Add `pub size: AirportSize` to the `Airport` struct.

### Build pipeline

In `build.rs`, carry the existing `airport_type` string through to the generated `airport_db.rs`, mapping:

- `"large_airport"` → `AirportSize::Large`
- `"medium_airport"` → `AirportSize::Medium`
- `"small_airport"` → `AirportSize::Small`

### Taxi time values

Taxi times are derived from departure and arrival airport sizes:

| Airport Size | Taxi Out (departure) | Taxi In (arrival) |
|---|---|---|
| Large | 18 min | 10 min |
| Medium | 10 min | 6 min |
| Small | 5 min | 3 min |

These values are calibrated against SimBrief dispatch data (e.g., SimBrief uses 20/8 for KELP-KDFW).

A function in `flight_plan.rs` computes taxi out/in from the departure and arrival airports:

```rust
fn taxi_times(departure: &Airport, arrival: &Airport) -> (Duration, Duration)
```

### FlightPlan struct changes

Replace `taxi_time: Duration` with:

- `taxi_out: Duration`
- `taxi_in: Duration`

Block time becomes: `climb_time + cruise_time + descent_time + taxi_out + taxi_in`.

### calculate_flight_plan signature change

Remove the `taxi_time: Duration` parameter. The function derives taxi times internally from the airports' sizes.

New signature:

```rust
pub fn calculate_flight_plan(
    departure: &'static Airport,
    arrival: &'static Airport,
    aircraft: &Aircraft,
) -> FlightPlan
```

### estimate_distance_for_block_time signature change

This function currently takes `taxi_time: Duration`. It changes to hardcode the medium-airport estimate (16 min) internally, removing the parameter:

```rust
pub fn estimate_distance_for_block_time(
    aircraft: &Aircraft,
    target_block_time: Duration,
) -> f64
```

### Selection algorithm

`FlightPlanOptions` drops the `taxi_time` field. `estimate_distance_for_block_time` uses a fixed medium-airport estimate (16 min total) as the search heuristic. Once a candidate pair is found, `calculate_flight_plan` computes real taxi times from actual airport categories. The existing tolerance window (default 15 min) absorbs the variance between estimated and actual taxi times.

Note: The worst-case taxi variance is 20 min (Large-Large 28 min vs Small-Small 8 min), giving up to 12 min error from the 16 min heuristic. This fits within the 15-min default tolerance with reduced headroom. For the typical use case (mixed airport sizes), the error is much smaller. If retries increase noticeably, the heuristic estimate can be tuned without API changes.

### CLI output

The taxi line in output splits from a single line to two:

```
Taxi Out:    18m
Taxi In:     10m
```

### Tests

- Remove `taxi()` helper and `taxi_time` parameter from `flight_plan.rs` tests.
- Update `block_time_equals_sum_of_phases` to check `climb + cruise + descent + taxi_out + taxi_in`.
- Remove `taxi_time` from `FlightPlanOptions` in `selection.rs` tests.
- Add test verifying taxi times vary by airport size (e.g., KJFK large departure vs small airport departure produce different taxi_out values).

## Breaking API changes

This is a breaking change to the public API. The following re-exports in `src/lib.rs` are affected:

- `calculate_flight_plan` — `taxi_time` parameter removed
- `estimate_distance_for_block_time` — `taxi_time` parameter removed
- `FlightPlan` — `taxi_time` replaced with `taxi_out` + `taxi_in`
- `FlightPlanOptions` — `taxi_time` field removed
- `Airport` — new `size` field added (additive, non-breaking)
- `AirportSize` — new public re-export

## Files changed

- `src/airport.rs` — add `AirportSize` enum and field to `Airport`
- `src/lib.rs` — add `AirportSize` re-export
- `build.rs` — emit `size` field in generated airport DB
- `src/flight_plan.rs` — add `taxi_times()`, update `calculate_flight_plan` and `estimate_distance_for_block_time` signatures, update `FlightPlan` struct
- `src/selection.rs` — remove `taxi_time` from options, use medium estimate in distance heuristic
- `src/main.rs` — display taxi out/in separately
