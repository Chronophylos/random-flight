# SimBrief Dispatch Link

## Summary

After generating a flight plan, append a SimBrief dispatch URL to the output. The URL pre-fills origin, destination, aircraft type, and cruise flight level so the user can open it in a browser and immediately plan the flight in SimBrief without re-entering details.

## Output

A new line is printed at the end of the existing flight plan output, after a blank separator line:

```
Flight Plan
===========
Aircraft:    B738 (Boeing 737-800)
Departure:   KJFK (John F. Kennedy International Airport)
Arrival:     EDDF (Frankfurt Main Airport)
Distance:    3342 nm
Block Time:  7h 50m

Cruise Alt:  36000 ft
Climb:       14m (74 nm)
Cruise:      7h 06m (3175 nm)
Descent:     19m (92 nm)
Taxi:        10m

SimBrief:    https://dispatch.simbrief.com/options/custom?orig=KJFK&dest=EDDF&type=B738&fl=36000
```

## URL Format

Base: `https://dispatch.simbrief.com/options/custom`

Query parameters:

| Parameter | Source | Example |
|-----------|--------|---------|
| `orig` | `departure.icao` | `KJFK` |
| `dest` | `arrival.icao` | `EDDF` |
| `type` | `aircraft.icao_type` | `B738` |
| `fl` | `cruise_altitude_ft` | `36000` |

All values are ASCII alphanumeric ICAO codes or integers — no URL encoding is needed.

## Implementation

### `flight_plan.rs`

Add a public method on `FlightPlan`:

```rust
pub fn simbrief_url(&self) -> String {
    format!(
        "https://dispatch.simbrief.com/options/custom?orig={}&dest={}&type={}&fl={}",
        self.departure.icao,
        self.arrival.icao,
        self.aircraft.icao_type,
        self.cruise_altitude_ft,
    )
}
```

### `main.rs`

In the `generate()` function, after the existing `println!` calls, add:

```rust
println!();
println!("SimBrief:    {}", plan.simbrief_url());
```

### Tests

- Unit test in `flight_plan.rs`: construct a `FlightPlan` with known values and assert the URL string matches the expected format.
- Integration test: run `cargo run -- generate --aircraft B738 --time 4h` and verify stdout contains a line starting with `SimBrief:    https://dispatch.simbrief.com/options/custom?`.

## Scope

- No new dependencies
- No new modules or files
- No changes to existing data structures
- Two small code additions (one method, one print block)
