# Aircraft Performance TOML Schema

This document defines the TOML format used by random-flight for aircraft performance profiles. Each `.toml` file in `data/aircraft/` describes one aircraft type and is compiled into the binary at build time via `build.rs`.

## Purpose

These profiles drive the flight model: cruise speed determines distance per hour, climb/descent rates determine phase durations, fuel capacity limits maximum range, and minimum runway length filters which airports are eligible.

## Complete Schema

```toml
[aircraft]
name = "string"            # Full display name (e.g. "Boeing 737-800")
icao_type = "string"       # ICAO type designator, 2-4 chars (e.g. "B738")

[performance]
cruise_speed_ktas = integer    # True airspeed at cruise altitude, knots (60-550)
cruise_altitude_ft = integer   # Typical cruise altitude, feet (1000-45000)
service_ceiling_ft = integer   # Maximum certified altitude, feet (5000-60000)
min_runway_length_ft = integer # Minimum required runway length, feet (800-15000)

[performance.climb]
speed_ktas = integer           # Climb speed, knots true airspeed (50-400)
rate_fpm = integer             # Rate of climb, feet per minute (200-6000)

[performance.descent]
speed_ktas = integer           # Descent speed, knots true airspeed (50-400)
rate_fpm = integer             # Rate of descent, feet per minute (200-6000)

[fuel]
capacity_kg = float            # Usable fuel capacity, kilograms (30.0-300000.0)
fuel_type = "string"           # Either "jet" or "avgas"

[fuel.flow]
climb_kg_per_hour = float      # Fuel flow during climb, kg/h
cruise_kg_per_hour = float     # Fuel flow during cruise, kg/h
descent_kg_per_hour = float    # Fuel flow during descent, kg/h
```

All fields are required. There are no optional fields.

## Field-by-Field Guidance

### `[aircraft]`

- **name**: Human-readable name shown in output. Use the manufacturer and common marketing name (e.g. "Cessna 172 Skyhawk", "Boeing 737-800", "Airbus A330-300").
- **icao_type**: The ICAO type designator used to look up aircraft. Must be unique across all profiles. Find these at [ICAO Doc 8643](https://www.icao.int/publications/doc8643) or in flight sim databases. Common examples: C172, B738, A320, B772.

### `[performance]`

- **cruise_speed_ktas**: Typical cruise true airspeed in knots. For GA piston aircraft this is usually 70-180 ktas; turboprops 200-360 ktas; jets 420-510 ktas. Source from the aircraft's POH/AFM or type certificate data.
- **cruise_altitude_ft**: The altitude the flight model will target for cruise. Use typical operational altitudes, not the ceiling. GA piston: 3000-12000; turboprops: 14000-35000; jets: 33000-41000. The flight model will automatically reduce this for short routes.
- **service_ceiling_ft**: Maximum altitude from the type certificate. This is an upper bound; `cruise_altitude_ft` should normally be lower.
- **min_runway_length_ft**: Minimum runway length needed for the airport filter. Use approximate takeoff distance at MTOW plus a safety margin. GA: 1500-3000; turboprops: 2500-4000; regional jets: 5000-6500; narrowbody: 5500-7000; widebody: 7500-10000.

### `[performance.climb]`

- **speed_ktas**: Typical climb speed. Usually lower than cruise speed. For jets, this is often 280-330 ktas; GA piston: 70-130 ktas.
- **rate_fpm**: Average rate of climb from sea level to cruise. Use a mid-altitude average, not the initial rate. GA piston: 500-1000; turboprops: 1000-2000; jets: 2000-3000.

### `[performance.descent]`

- **speed_ktas**: Typical descent speed. Often similar to or slightly lower than climb speed.
- **rate_fpm**: Average rate of descent from cruise to pattern altitude. A standard jet descent is roughly 1500-2000 fpm; GA: 400-800 fpm.

### `[fuel]`

- **capacity_kg**: Total usable fuel capacity in kilograms. See conversion reference below if your source uses pounds, gallons, or liters.
- **fuel_type**: `"jet"` for Jet-A/JP fuel (turbine engines), `"avgas"` for 100LL aviation gasoline (piston engines). This affects density calculations and the airport fuel availability filter (future feature).

### `[fuel.flow]`

- **climb_kg_per_hour**: Fuel burn rate during climb phase. Typically the highest of the three phases.
- **cruise_kg_per_hour**: Fuel burn rate during cruise. This is the steady-state value at cruise altitude and speed.
- **descent_kg_per_hour**: Fuel burn rate during descent. Typically the lowest, as engines are at reduced power.

Flow rates should satisfy: `climb > cruise > descent`. If your source gives total fuel per phase, divide by the estimated phase duration to get an hourly rate.

## Annotated Example: Boeing 737-800

```toml
[aircraft]
name = "Boeing 737-800"       # Official marketing name
icao_type = "B738"             # ICAO Doc 8643 designator

[performance]
cruise_speed_ktas = 460        # M0.785 at FL360 ≈ 460 KTAS
cruise_altitude_ft = 36000     # Typical FL360 for medium-haul
service_ceiling_ft = 41000     # Per type certificate
min_runway_length_ft = 6000    # ~5800 ft MTOW takeoff + margin

[performance.climb]
speed_ktas = 310               # 310 KTAS average through climb
rate_fpm = 2500                # Mid-altitude average rate

[performance.descent]
speed_ktas = 280               # Standard descent profile
rate_fpm = 1800                # ~3 degree path at this speed

[fuel]
capacity_kg = 20894.0          # 46,063 lbs = 6,875 US gal × 6.7 lbs/gal
fuel_type = "jet"              # Jet-A

[fuel.flow]
climb_kg_per_hour = 3402.0     # High thrust during climb
cruise_kg_per_hour = 2359.0    # ~5,200 lbs/h at cruise
descent_kg_per_hour = 1270.0   # Idle descent
```

## Conversion Reference

Use these when your data source reports in non-metric units:

| Conversion | Factor |
|---|---|
| Pounds (lbs) to kilograms (kg) | multiply by 0.453592 |
| US gallons (gal) to liters (L) | multiply by 3.78541 |
| Jet-A density | 0.804 kg/L or 6.7 lbs/US gal |
| Avgas (100LL) density | 0.721 kg/L or 6.02 lbs/US gal |

**Common conversions for fuel capacity:**
- Gallons of Jet-A to kg: gallons x 3.78541 x 0.804 (or gallons x 3.043)
- Pounds of fuel to kg: pounds x 0.453592
- Liters of Jet-A to kg: liters x 0.804

## Tips by Aircraft Category

### GA Piston (C152, C172, SR22)
- Cruise altitudes are low (3,000-12,000 ft); use MSL feet, not flight levels.
- Fuel capacity is small (80-300 kg). Double-check unit conversions.
- Fuel type is `"avgas"`.
- Climb and descent rates are modest (500-1000 fpm).
- Minimum runway lengths are short (1500-3000 ft).

### Turboprop (C208, TBM9, B350)
- Cruise at higher altitudes (14,000-35,000 ft) but vary widely by type.
- Fuel type is `"jet"` (turbine engines burn Jet-A).
- Performance spans a wide range: a Caravan cruises at 186 ktas, a TBM at 330 ktas.
- Climb rates are moderate (1000-2000 fpm).

### Regional and Narrowbody Jets (CRJ7, E190, A320, B738)
- Cruise in the flight levels (FL330-FL410), speeds 440-470 ktas.
- Fuel type is `"jet"`.
- Climb/descent speeds 280-330 ktas, rates 2000-3000 fpm.
- Minimum runway 5000-7000 ft.

### Widebody Jets (B772, B789, A333, A388)
- Cruise FL380-FL410, speeds 470-510 ktas.
- Very large fuel capacities (100,000-250,000 kg).
- Minimum runway 7500-10000 ft.
- Fuel flows are high (5000-14000 kg/h cruise).

## Instructions for AI Producing a Profile

1. **Identify the aircraft** by its ICAO type designator and full name.
2. **Determine fuel type**: piston engine = `"avgas"`, turbine engine = `"jet"`.
3. **Look up published performance data** from the manufacturer's specification sheet, POH, or a reliable aviation database. Key values needed: cruise speed, cruise altitude, service ceiling, fuel capacity, fuel flow rates at climb/cruise/descent power settings.
4. **Convert all fuel values to kilograms** using the conversion table above. Fuel capacity should be the usable fuel amount.
5. **Estimate minimum runway length** from published takeoff distances at MTOW. Add a margin of 10-20%.
6. **Use averages for climb/descent rates**: climb rate should be an average from sea level to cruise, not the initial max rate. Descent rate should represent a standard 2.5-3 degree descent path.
7. **Verify the constraint**: climb fuel flow > cruise fuel flow > descent fuel flow. If this does not hold, re-check your sources.
8. **Use the exact TOML structure** shown in the schema above. All fields are required. Use the file naming convention: lowercase ICAO designator + `.toml` (e.g., `b738.toml`).
9. **Validate** by running `cargo build` -- the build will fail if any field is missing or has the wrong type.
