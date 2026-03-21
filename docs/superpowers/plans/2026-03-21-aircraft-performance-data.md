# Aircraft Performance Data Files — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move aircraft performance data from hardcoded Rust source into TOML files, with compile-time embedding, runtime custom profile loading, and LNM import.

**Architecture:** TOML files in `data/aircraft/` are parsed by `build.rs` to generate a static `Aircraft` array (same pattern as airports). A new `--profile` CLI flag loads custom profiles at runtime. A new `aircraft import` subcommand converts LNM `.lnmperf` files to our TOML format.

**Tech Stack:** Rust, TOML (serde), quick-xml, clap (derive)

**Spec:** `docs/superpowers/specs/2026-03-21-aircraft-performance-data-design.md`

---

### Task 1: Update Aircraft struct, FuelType enum, and all consumers

**Files:**
- Modify: `src/aircraft.rs`
- Modify: `src/lib.rs`
- Modify: `src/flight_plan.rs`
- Modify: `src/selection.rs`
- Modify: `src/error.rs`

This task updates the core data model AND all consuming code atomically so the project stays compilable at every commit. The hardcoded `BUILT_IN` array is kept temporarily (with fields adapted). Later tasks will replace it with generated code.

- [ ] **Step 1: Add FuelType enum and update Aircraft struct**

In `src/aircraft.rs`, replace the existing struct and add FuelType:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FuelType {
    Jet,   // Jet-A, density 0.804 kg/L
    Avgas, // 100LL, density 0.721 kg/L
}

#[derive(Debug, Clone)]
pub struct Aircraft {
    pub name: &'static str,
    pub icao_type: &'static str,
    pub cruise_speed_ktas: u16,
    pub cruise_altitude_ft: u32,
    pub service_ceiling_ft: u32,
    pub min_runway_length_ft: u32,
    pub climb_speed_ktas: u16,
    pub climb_rate_fpm: u16,
    pub descent_speed_ktas: u16,
    pub descent_rate_fpm: u16,
    pub fuel_capacity_kg: f64,
    pub fuel_type: FuelType,
    pub fuel_flow_climb_kg_per_hour: f64,
    pub fuel_flow_cruise_kg_per_hour: f64,
    pub fuel_flow_descent_kg_per_hour: f64,
}
```

- [ ] **Step 2: Add range_nm() method**

```rust
impl Aircraft {
    /// Derived max range with 5% fuel reserve.
    pub fn range_nm(&self) -> f64 {
        (self.fuel_capacity_kg * 0.95 / self.fuel_flow_cruise_kg_per_hour)
            * self.cruise_speed_ktas as f64
    }
}
```

- [ ] **Step 3: Rename aircraft_by_name to aircraft_by_icao_type**

```rust
pub fn aircraft_by_icao_type(icao_type: &str) -> Option<&'static Aircraft> {
    BUILT_IN.iter().find(|a| a.icao_type.eq_ignore_ascii_case(icao_type))
}
```

Keep `aircraft_by_name` as a deprecated alias temporarily — remove it in Task 6 when all call sites are updated.

- [ ] **Step 4: Update BUILT_IN array with new fields**

Convert the 6 existing aircraft to the new struct format. Use realistic values derived from the current data plus reasonable fuel estimates. Example for B738:

```rust
Aircraft {
    name: "Boeing 737-800",
    icao_type: "B738",
    cruise_speed_ktas: 460,
    cruise_altitude_ft: 36000,
    service_ceiling_ft: 41000,
    min_runway_length_ft: 6000,
    climb_speed_ktas: 310,
    climb_rate_fpm: 2500,
    descent_speed_ktas: 280,
    descent_rate_fpm: 1800,
    fuel_capacity_kg: 20894.0,
    fuel_type: FuelType::Jet,
    fuel_flow_climb_kg_per_hour: 3402.0,
    fuel_flow_cruise_kg_per_hour: 2359.0,
    fuel_flow_descent_kg_per_hour: 1270.0,
},
```

For the climb/descent speeds: use the old `cruise_speed * factor` to compute absolute values for now (e.g., B738: 460 * 0.75 = 345 → round to 310 for a more realistic value). These will be refined when TOML files replace the hardcoded data.

Full preset conversions (approximate values to keep existing behavior close):

| Aircraft | climb_speed_ktas | descent_speed_ktas | fuel_cap_kg | cruise_flow | climb_flow | descent_flow | ceiling |
|----------|------------------|--------------------|-------------|-------------|------------|--------------|---------|
| C172 | 80 | 90 | 109.0 | 21.0 | 30.0 | 16.0 | 14000 |
| C208 | 140 | 120 | 1000.0 | 160.0 | 200.0 | 100.0 | 25000 |
| B738 | 310 | 280 | 20894.0 | 2359.0 | 3402.0 | 1270.0 | 41000 |
| A320 | 310 | 280 | 19144.0 | 2300.0 | 3300.0 | 1200.0 | 41000 |
| A388 | 330 | 300 | 253983.0 | 10000.0 | 14000.0 | 5500.0 | 45000 |
| CRJ7 | 310 | 280 | 8875.0 | 1600.0 | 2200.0 | 900.0 | 41000 |

- [ ] **Step 5: Update lib.rs exports**

In `src/lib.rs`, update the re-exports:

```rust
pub use aircraft::{Aircraft, FuelType, aircraft_by_icao_type, built_in_aircraft};
```

Keep `aircraft_by_name` re-exported temporarily for backward compatibility during migration.

- [ ] **Step 6: Update Error::RangeExceeded to use f64**

In `src/error.rs`, change `range_nm: u32` to `range_nm: f64` — this must be done before updating `selection.rs`:

```rust
#[error("flight distance {distance_nm:.0} nm exceeds aircraft range of {range_nm:.0} nm")]
RangeExceeded { distance_nm: f64, range_nm: f64 },
```

- [ ] **Step 7: Update flight_plan.rs to use new field names**

In `calculate_flight_plan()`, replace:
- `aircraft.cruise_speed_kts` → `aircraft.cruise_speed_ktas`
- `cruise_speed * aircraft.climb_speed_factor as f64` → `aircraft.climb_speed_ktas as f64`
- `cruise_speed * aircraft.descent_speed_factor as f64` → `aircraft.descent_speed_ktas as f64`

The full updated speed lines:

```rust
let cruise_speed = aircraft.cruise_speed_ktas as f64;
let climb_speed = aircraft.climb_speed_ktas as f64;
let descent_speed = aircraft.descent_speed_ktas as f64;
```

In `estimate_distance_for_block_time()`:
- `aircraft.cruise_speed_kts` → `aircraft.cruise_speed_ktas`

- [ ] **Step 8: Update selection.rs to use range_nm() method**

Replace `aircraft.range_nm as f64` with `aircraft.range_nm()` in the candidate filter (line 89):

```rust
d >= min_dist && d <= max_dist && d <= aircraft.range_nm()
```

And in `plan_for_pair()` (line 158):

```rust
if distance > aircraft.range_nm() {
    return Err(Error::RangeExceeded {
        distance_nm: distance,
        range_nm: aircraft.range_nm(),
    });
}
```

Also update `dist_margin` (line 60):

```rust
let dist_margin = aircraft.cruise_speed_ktas as f64 * tolerance_hrs;
```

- [ ] **Step 9: Update all tests in aircraft.rs, flight_plan.rs, and selection.rs**

Replace all `aircraft_by_name` with `aircraft_by_icao_type` across test code.

Updated aircraft.rs tests:

```rust
#[test]
fn c172_preset_exists() {
    let a = aircraft_by_icao_type("C172").expect("C172 should exist");
    assert_eq!(a.cruise_speed_ktas, 122);
    assert_eq!(a.min_runway_length_ft, 2000);
}

#[test]
fn b738_preset_exists() {
    let a = aircraft_by_icao_type("B738").expect("B738 should exist");
    assert!(a.cruise_speed_ktas > 400);
    assert!(a.range_nm() > 2000.0);
}

#[test]
fn case_insensitive_lookup() {
    assert!(aircraft_by_icao_type("c172").is_some());
    assert!(aircraft_by_icao_type("C172").is_some());
}

#[test]
fn unknown_aircraft_returns_none() {
    assert!(aircraft_by_icao_type("ZZZZ").is_none());
}

#[test]
fn range_derived_from_fuel() {
    let a = aircraft_by_icao_type("B738").expect("B738");
    let expected = (a.fuel_capacity_kg * 0.95 / a.fuel_flow_cruise_kg_per_hour)
        * a.cruise_speed_ktas as f64;
    assert!((a.range_nm() - expected).abs() < 0.01);
}
```

- [ ] **Step 10: Verify library compiles and tests pass**

Run: `cargo test --lib`
Expected: PASS

- [ ] **Step 11: Commit**

```bash
git add src/aircraft.rs src/lib.rs src/flight_plan.rs src/selection.rs src/error.rs
git commit -m "refactor: update Aircraft struct with per-phase speeds and fuel data, update all consumers"
```

---

### Task 2: Update CLI (main.rs) and integration tests

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/integration.rs`

- [ ] **Step 1: Update Commands enum for aircraft subcommands**

Replace the flat `Aircraft` variant with a nested subcommand:

```rust
#[derive(Subcommand)]
enum Commands {
    /// Generate a random flight plan
    #[command(
        after_help = "Examples:\n  \
            random-flight generate --aircraft B738 --time 4h\n  \
            random-flight generate --aircraft C172 --time 1h30m --departure KJFK\n  \
            random-flight generate --profile custom.toml --time 3h"
    )]
    Generate(GenerateArgs),
    /// Aircraft presets and import tools
    #[command(subcommand)]
    Aircraft(AircraftCommands),
}

#[derive(Subcommand)]
enum AircraftCommands {
    /// List available aircraft presets
    List,
    /// Import aircraft performance from external format
    Import(ImportArgs),
}

#[derive(Parser)]
struct ImportArgs {
    /// Source format
    #[arg(long)]
    format: String,

    /// Input file path
    input: String,

    /// Output directory
    #[arg(long, default_value = ".")]
    output: String,
}
```

- [ ] **Step 2: Update GenerateArgs — remove custom aircraft flags, add --profile**

```rust
#[derive(Parser)]
struct GenerateArgs {
    /// Aircraft preset name (e.g. C172, B738, A320)
    #[arg(long)]
    aircraft: Option<String>,

    /// Path to custom aircraft TOML profile
    #[arg(long, conflicts_with = "aircraft")]
    profile: Option<String>,

    /// Target block time (e.g. 2h, 2h30m, 90m)
    #[arg(long)]
    time: String,

    /// Tolerance around target time
    #[arg(long, default_value = "15m")]
    tolerance: String,

    /// Pin departure airport (ICAO code)
    #[arg(long)]
    departure: Option<String>,

    /// Pin arrival airport (ICAO code)
    #[arg(long)]
    arrival: Option<String>,
}
```

Note: `aircraft` and `profile` are both `Option`, with `conflicts_with` enforcing mutual exclusivity. At least one must be provided — validated in `resolve_aircraft()`.

- [ ] **Step 3: Update main() match and resolve_aircraft()**

```rust
fn main() {
    let mut cmd = <Cli as clap::CommandFactory>::command();
    if std::env::args_os().len() == 1 {
        let _ = cmd.print_help();
        println!();
        return;
    }

    let cli = Cli::parse();

    match cli.command {
        Commands::Aircraft(sub) => match sub {
            AircraftCommands::List => list_aircraft(),
            AircraftCommands::Import(args) => import_aircraft(args),
        },
        Commands::Generate(args) => generate(args),
    }
}
```

Update `resolve_aircraft()`:

```rust
fn resolve_aircraft(args: &GenerateArgs) -> Aircraft {
    if let Some(ref path) = args.profile {
        // Runtime profile loading — implemented in Task 5
        eprintln!("Error: --profile not yet implemented");
        process::exit(1);
    } else if let Some(ref name) = args.aircraft {
        match aircraft_by_icao_type(name) {
            Some(a) => a.clone(),
            None => {
                eprintln!("Error: unknown aircraft '{name}'. Run `random-flight aircraft list` to see presets.");
                process::exit(1);
            }
        }
    } else {
        eprintln!("Error: either --aircraft or --profile is required");
        process::exit(1);
    }
}
```

- [ ] **Step 4: Add placeholder import_aircraft()**

```rust
fn import_aircraft(args: ImportArgs) {
    eprintln!("Error: import not yet implemented (format: {})", args.format);
    process::exit(1);
}
```

- [ ] **Step 5: Update list_aircraft() for new fields**

```rust
fn list_aircraft() {
    println!("Available aircraft presets:\n");
    println!("  {:<6} {:<22} {:>5}  {:>7}  {:>8}  {:>10}",
        "ICAO", "NAME", "SPD", "ALT", "RANGE", "MIN RWY");
    println!("  {:<6} {:<22} {:>5}  {:>7}  {:>8}  {:>10}",
        "----", "----", "---", "---", "-----", "-------");
    for a in built_in_aircraft() {
        println!("  {:<6} {:<22} {:>3} kt  FL{:03}    {:>5.0} nm  {:>6} ft",
            a.icao_type, a.name, a.cruise_speed_ktas, a.cruise_altitude_ft / 100,
            a.range_nm(), a.min_runway_length_ft);
    }
}
```

- [ ] **Step 6: Update main.rs imports**

```rust
use random_flight::{
    Aircraft, FlightPlanOptions, aircraft_by_icao_type, built_in_aircraft,
    generate_flight_plan,
};
```

- [ ] **Step 7: Update Cli help text**

Update the `after_help` in the `Cli` struct:

```rust
after_help = "Examples:\n  \
    random-flight generate --aircraft B738 --time 4h\n  \
    random-flight generate --aircraft C172 --time 1h30m --departure KJFK\n  \
    random-flight aircraft list",
```

- [ ] **Step 8: Update FlightPlan output to show icao_type**

In the `generate()` function, change:
```rust
println!("Aircraft:    {} ({})", fp.aircraft.icao_type, fp.aircraft.name);
```

- [ ] **Step 9: Update integration tests**

In `tests/integration.rs`:

Replace all `aircraft_by_name` with `aircraft_by_icao_type` in imports and calls.

Update `cli_aircraft_lists_presets` test:

```rust
#[test]
fn cli_aircraft_lists_presets() {
    let bin = env!("CARGO_BIN_EXE_random-flight");
    let output = std::process::Command::new(bin)
        .args(["aircraft", "list"])
        .output()
        .expect("failed to run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("C172"), "expected C172 in aircraft list");
    assert!(stdout.contains("B738"), "expected B738 in aircraft list");
}
```

Update `cli_no_subcommand_shows_help` to check for "aircraft" and "generate" still.

- [ ] **Step 10: Run all tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 11: Run clippy**

Run: `cargo clippy`
Expected: no warnings

- [ ] **Step 12: Commit**

```bash
git add src/main.rs src/lib.rs tests/integration.rs
git commit -m "refactor: update CLI with aircraft subcommands, --profile flag, remove inline custom aircraft"
```

---

### Task 3: Create TOML files and build.rs integration

**Files:**
- Create: `data/aircraft/b738.toml` (and 5 more for existing presets)
- Modify: `build.rs`
- Modify: `src/aircraft.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add toml and serde to build-dependencies**

In `Cargo.toml`, add under `[build-dependencies]`:

```toml
toml = "0.8"
serde = { version = "1", features = ["derive"] }
```

Note: `serde` is already there as a build-dependency. Just add `toml`.

- [ ] **Step 2: Create the 6 initial TOML files**

Create `data/aircraft/c172.toml`:

```toml
[aircraft]
name = "Cessna 172 Skyhawk"
icao_type = "C172"

[performance]
cruise_speed_ktas = 122
cruise_altitude_ft = 8000
service_ceiling_ft = 14000
min_runway_length_ft = 2000

[performance.climb]
speed_ktas = 80
rate_fpm = 700

[performance.descent]
speed_ktas = 90
rate_fpm = 500

[fuel]
capacity_kg = 109.0
fuel_type = "avgas"

[fuel.flow]
climb_kg_per_hour = 30.0
cruise_kg_per_hour = 21.0
descent_kg_per_hour = 16.0
```

Create `data/aircraft/c208.toml`:

```toml
[aircraft]
name = "Cessna 208B Grand Caravan"
icao_type = "C208"

[performance]
cruise_speed_ktas = 186
cruise_altitude_ft = 14000
service_ceiling_ft = 25000
min_runway_length_ft = 3000

[performance.climb]
speed_ktas = 140
rate_fpm = 1000

[performance.descent]
speed_ktas = 120
rate_fpm = 800

[fuel]
capacity_kg = 1000.0
fuel_type = "jet"

[fuel.flow]
climb_kg_per_hour = 200.0
cruise_kg_per_hour = 160.0
descent_kg_per_hour = 100.0
```

Create `data/aircraft/b738.toml`:

```toml
[aircraft]
name = "Boeing 737-800"
icao_type = "B738"

[performance]
cruise_speed_ktas = 460
cruise_altitude_ft = 36000
service_ceiling_ft = 41000
min_runway_length_ft = 6000

[performance.climb]
speed_ktas = 310
rate_fpm = 2500

[performance.descent]
speed_ktas = 280
rate_fpm = 1800

[fuel]
capacity_kg = 20894.0
fuel_type = "jet"

[fuel.flow]
climb_kg_per_hour = 3402.0
cruise_kg_per_hour = 2359.0
descent_kg_per_hour = 1270.0
```

Create `data/aircraft/a320.toml`:

```toml
[aircraft]
name = "Airbus A320"
icao_type = "A320"

[performance]
cruise_speed_ktas = 447
cruise_altitude_ft = 36000
service_ceiling_ft = 41000
min_runway_length_ft = 6000

[performance.climb]
speed_ktas = 310
rate_fpm = 2500

[performance.descent]
speed_ktas = 280
rate_fpm = 1800

[fuel]
capacity_kg = 19144.0
fuel_type = "jet"

[fuel.flow]
climb_kg_per_hour = 3300.0
cruise_kg_per_hour = 2300.0
descent_kg_per_hour = 1200.0
```

Create `data/aircraft/a388.toml`:

```toml
[aircraft]
name = "Airbus A380-800"
icao_type = "A388"

[performance]
cruise_speed_ktas = 480
cruise_altitude_ft = 40000
service_ceiling_ft = 45000
min_runway_length_ft = 9000

[performance.climb]
speed_ktas = 330
rate_fpm = 2000

[performance.descent]
speed_ktas = 300
rate_fpm = 1500

[fuel]
capacity_kg = 253983.0
fuel_type = "jet"

[fuel.flow]
climb_kg_per_hour = 14000.0
cruise_kg_per_hour = 10000.0
descent_kg_per_hour = 5500.0
```

Create `data/aircraft/crj7.toml`:

```toml
[aircraft]
name = "Bombardier CRJ-700"
icao_type = "CRJ7"

[performance]
cruise_speed_ktas = 447
cruise_altitude_ft = 37000
service_ceiling_ft = 41000
min_runway_length_ft = 5500

[performance.climb]
speed_ktas = 310
rate_fpm = 2500

[performance.descent]
speed_ktas = 280
rate_fpm = 1800

[fuel]
capacity_kg = 8875.0
fuel_type = "jet"

[fuel.flow]
climb_kg_per_hour = 2200.0
cruise_kg_per_hour = 1600.0
descent_kg_per_hour = 900.0
```

- [ ] **Step 3: Add TOML parsing to build.rs**

Add a new section to `build.rs` after the airport generation. Add serde structs and the generation logic:

```rust
// At the top, add:
use toml;

// Serde structs for TOML deserialization
#[derive(Debug, Deserialize)]
struct AircraftToml {
    aircraft: AircraftMeta,
    performance: PerformanceToml,
    fuel: FuelToml,
}

#[derive(Debug, Deserialize)]
struct AircraftMeta {
    name: String,
    icao_type: String,
}

#[derive(Debug, Deserialize)]
struct PerformanceToml {
    cruise_speed_ktas: u16,
    cruise_altitude_ft: u32,
    service_ceiling_ft: u32,
    min_runway_length_ft: u32,
    climb: PhaseToml,
    descent: PhaseToml,
}

#[derive(Debug, Deserialize)]
struct PhaseToml {
    speed_ktas: u16,
    rate_fpm: u16,
}

#[derive(Debug, Deserialize)]
struct FuelToml {
    capacity_kg: f64,
    fuel_type: String,
    flow: FuelFlowToml,
}

#[derive(Debug, Deserialize)]
struct FuelFlowToml {
    climb_kg_per_hour: f64,
    cruise_kg_per_hour: f64,
    descent_kg_per_hour: f64,
}
```

Add the generation function:

```rust
fn generate_aircraft_db(out_dir: &Path, manifest_dir: &Path) {
    let aircraft_dir = manifest_dir.join("data").join("aircraft");
    let mut aircraft: Vec<AircraftToml> = Vec::new();
    let mut seen_types: HashMap<String, String> = HashMap::new(); // icao_type -> filename

    let mut entries: Vec<_> = fs::read_dir(&aircraft_dir)
        .expect("failed to read data/aircraft/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "toml"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let parsed: AircraftToml = toml::from_str(&contents)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));

        // Validate filename matches icao_type
        let expected_filename = format!("{}.toml", parsed.aircraft.icao_type.to_lowercase());
        assert_eq!(
            filename, expected_filename,
            "Filename {filename} does not match icao_type {} (expected {expected_filename})",
            parsed.aircraft.icao_type
        );

        // Check for duplicate icao_type
        if let Some(prev) = seen_types.get(&parsed.aircraft.icao_type) {
            panic!(
                "Duplicate icao_type '{}' in {} and {prev}",
                parsed.aircraft.icao_type, filename
            );
        }
        seen_types.insert(parsed.aircraft.icao_type.clone(), filename.clone());

        // Validate ranges
        assert!(parsed.performance.cruise_speed_ktas > 0, "{filename}: cruise_speed_ktas must be > 0");
        assert!(parsed.performance.service_ceiling_ft >= parsed.performance.cruise_altitude_ft,
            "{filename}: service_ceiling_ft must be >= cruise_altitude_ft");
        assert!(parsed.fuel.capacity_kg > 0.0, "{filename}: fuel capacity must be > 0");
        assert!(parsed.fuel.fuel_type == "jet" || parsed.fuel.fuel_type == "avgas",
            "{filename}: fuel_type must be 'jet' or 'avgas'");

        println!("cargo:rerun-if-changed=data/aircraft/{filename}");
        aircraft.push(parsed);
    }

    // Generate Rust source
    let gen_path = out_dir.join("aircraft_db.rs");
    let mut f = fs::File::create(&gen_path).expect("create aircraft_db.rs");

    writeln!(f, "static AIRCRAFT_DB: &[Aircraft] = &[").unwrap();
    for ac in &aircraft {
        let fuel_type = match ac.fuel.fuel_type.as_str() {
            "jet" => "FuelType::Jet",
            "avgas" => "FuelType::Avgas",
            _ => unreachable!(),
        };
        writeln!(f, "    Aircraft {{").unwrap();
        writeln!(f, "        name: \"{}\",", ac.aircraft.name.replace('"', "\\\"")).unwrap();
        writeln!(f, "        icao_type: \"{}\",", ac.aircraft.icao_type).unwrap();
        writeln!(f, "        cruise_speed_ktas: {},", ac.performance.cruise_speed_ktas).unwrap();
        writeln!(f, "        cruise_altitude_ft: {},", ac.performance.cruise_altitude_ft).unwrap();
        writeln!(f, "        service_ceiling_ft: {},", ac.performance.service_ceiling_ft).unwrap();
        writeln!(f, "        min_runway_length_ft: {},", ac.performance.min_runway_length_ft).unwrap();
        writeln!(f, "        climb_speed_ktas: {},", ac.performance.climb.speed_ktas).unwrap();
        writeln!(f, "        climb_rate_fpm: {},", ac.performance.climb.rate_fpm).unwrap();
        writeln!(f, "        descent_speed_ktas: {},", ac.performance.descent.speed_ktas).unwrap();
        writeln!(f, "        descent_rate_fpm: {},", ac.performance.descent.rate_fpm).unwrap();
        writeln!(f, "        fuel_capacity_kg: {:.1},", ac.fuel.capacity_kg).unwrap();
        writeln!(f, "        fuel_type: {fuel_type},").unwrap();
        writeln!(f, "        fuel_flow_climb_kg_per_hour: {:.1},", ac.fuel.flow.climb_kg_per_hour).unwrap();
        writeln!(f, "        fuel_flow_cruise_kg_per_hour: {:.1},", ac.fuel.flow.cruise_kg_per_hour).unwrap();
        writeln!(f, "        fuel_flow_descent_kg_per_hour: {:.1},", ac.fuel.flow.descent_kg_per_hour).unwrap();
        writeln!(f, "    }},").unwrap();
    }
    writeln!(f, "];").unwrap();

    println!("cargo:rerun-if-changed=data/aircraft/");
    println!("cargo:warning=Generated aircraft DB with {} aircraft", aircraft.len());
}
```

Call `generate_aircraft_db(&out_dir, &manifest_dir);` at the end of `main()` in build.rs.

- [ ] **Step 4: Replace hardcoded BUILT_IN with generated include**

In `src/aircraft.rs`, replace the entire `static BUILT_IN: &[Aircraft] = &[...];` block with:

```rust
include!(concat!(env!("OUT_DIR"), "/aircraft_db.rs"));
```

And update the lookup functions to use `AIRCRAFT_DB`:

```rust
pub fn built_in_aircraft() -> &'static [Aircraft] {
    AIRCRAFT_DB
}

pub fn aircraft_by_icao_type(icao_type: &str) -> Option<&'static Aircraft> {
    AIRCRAFT_DB.iter().find(|a| a.icao_type.eq_ignore_ascii_case(icao_type))
}
```

Remove the deprecated `aircraft_by_name` function and its re-export from `lib.rs`.

- [ ] **Step 5: Build and test**

Run: `cargo build 2>&1`
Expected: PASS — build.rs generates aircraft_db.rs from TOML files

Run: `cargo test`
Expected: PASS

- [ ] **Step 6: Run clippy**

Run: `cargo clippy`
Expected: no warnings

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml build.rs src/aircraft.rs src/lib.rs data/aircraft/
git commit -m "feat: generate aircraft database from TOML files at build time"
```

---

### Task 4: Runtime custom profile loading (--profile flag)

**Files:**
- Create: `src/profile.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Write the failing test for profile loading**

Create `src/profile.rs`:

```rust
use std::path::Path;

use serde::Deserialize;

use crate::aircraft::{Aircraft, FuelType};

#[derive(Debug, Deserialize)]
struct AircraftToml {
    aircraft: AircraftMeta,
    performance: PerformanceToml,
    fuel: FuelToml,
}

#[derive(Debug, Deserialize)]
struct AircraftMeta {
    name: String,
    icao_type: String,
}

#[derive(Debug, Deserialize)]
struct PerformanceToml {
    cruise_speed_ktas: u16,
    cruise_altitude_ft: u32,
    service_ceiling_ft: u32,
    min_runway_length_ft: u32,
    climb: PhaseToml,
    descent: PhaseToml,
}

#[derive(Debug, Deserialize)]
struct PhaseToml {
    speed_ktas: u16,
    rate_fpm: u16,
}

#[derive(Debug, Deserialize)]
struct FuelToml {
    capacity_kg: f64,
    fuel_type: String,
    flow: FuelFlowToml,
}

#[derive(Debug, Deserialize)]
struct FuelFlowToml {
    climb_kg_per_hour: f64,
    cruise_kg_per_hour: f64,
    descent_kg_per_hour: f64,
}

pub fn load_profile(path: &Path) -> Result<Aircraft, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    parse_profile(&contents)
}

fn parse_profile(contents: &str) -> Result<Aircraft, String> {
    let parsed: AircraftToml = toml::from_str(contents)
        .map_err(|e| format!("failed to parse TOML: {e}"))?;

    let fuel_type = match parsed.fuel.fuel_type.as_str() {
        "jet" => FuelType::Jet,
        "avgas" => FuelType::Avgas,
        other => return Err(format!("unknown fuel_type: {other}")),
    };

    if parsed.performance.service_ceiling_ft < parsed.performance.cruise_altitude_ft {
        return Err("service_ceiling_ft must be >= cruise_altitude_ft".into());
    }

    // Leak strings to get 'static lifetime — acceptable for one-time startup load
    let name: &'static str = Box::leak(parsed.aircraft.name.into_boxed_str());
    let icao_type: &'static str = Box::leak(parsed.aircraft.icao_type.into_boxed_str());

    Ok(Aircraft {
        name,
        icao_type,
        cruise_speed_ktas: parsed.performance.cruise_speed_ktas,
        cruise_altitude_ft: parsed.performance.cruise_altitude_ft,
        service_ceiling_ft: parsed.performance.service_ceiling_ft,
        min_runway_length_ft: parsed.performance.min_runway_length_ft,
        climb_speed_ktas: parsed.performance.climb.speed_ktas,
        climb_rate_fpm: parsed.performance.climb.rate_fpm,
        descent_speed_ktas: parsed.performance.descent.speed_ktas,
        descent_rate_fpm: parsed.performance.descent.rate_fpm,
        fuel_capacity_kg: parsed.fuel.capacity_kg,
        fuel_type,
        fuel_flow_climb_kg_per_hour: parsed.fuel.flow.climb_kg_per_hour,
        fuel_flow_cruise_kg_per_hour: parsed.fuel.flow.cruise_kg_per_hour,
        fuel_flow_descent_kg_per_hour: parsed.fuel.flow.descent_kg_per_hour,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TOML: &str = r#"
[aircraft]
name = "Test Aircraft"
icao_type = "TEST"

[performance]
cruise_speed_ktas = 250
cruise_altitude_ft = 25000
service_ceiling_ft = 30000
min_runway_length_ft = 4000

[performance.climb]
speed_ktas = 200
rate_fpm = 2000

[performance.descent]
speed_ktas = 180
rate_fpm = 1500

[fuel]
capacity_kg = 5000.0
fuel_type = "jet"

[fuel.flow]
climb_kg_per_hour = 1500.0
cruise_kg_per_hour = 1000.0
descent_kg_per_hour = 600.0
"#;

    #[test]
    fn parse_valid_profile() {
        let ac = parse_profile(VALID_TOML).expect("should parse");
        assert_eq!(ac.icao_type, "TEST");
        assert_eq!(ac.cruise_speed_ktas, 250);
        assert_eq!(ac.fuel_type, FuelType::Jet);
        assert!((ac.fuel_capacity_kg - 5000.0).abs() < 0.01);
    }

    #[test]
    fn parse_invalid_fuel_type() {
        let toml = VALID_TOML.replace("\"jet\"", "\"diesel\"");
        let result = parse_profile(&toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("fuel_type"));
    }

    #[test]
    fn parse_ceiling_below_cruise() {
        let toml = VALID_TOML.replace("service_ceiling_ft = 30000", "service_ceiling_ft = 20000");
        let result = parse_profile(&toml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("ceiling"));
    }

    #[test]
    fn parse_missing_field() {
        let toml = "[aircraft]\nname = \"X\"\nicao_type = \"X\"\n";
        let result = parse_profile(toml);
        assert!(result.is_err());
    }

    #[test]
    fn round_trip_matches_built_in() {
        // Load the b738.toml file and compare with built-in
        let path = std::path::Path::new("data/aircraft/b738.toml");
        if path.exists() {
            let loaded = load_profile(path).expect("should load b738.toml");
            let built_in = crate::aircraft::aircraft_by_icao_type("B738").expect("B738");
            assert_eq!(loaded.cruise_speed_ktas, built_in.cruise_speed_ktas);
            assert_eq!(loaded.climb_rate_fpm, built_in.climb_rate_fpm);
            assert!((loaded.fuel_capacity_kg - built_in.fuel_capacity_kg).abs() < 0.1);
        }
    }
}
```

- [ ] **Step 2: Add toml and serde to regular dependencies**

In `Cargo.toml`, add under `[dependencies]` (note: `serde` is already in `[build-dependencies]` — it needs to be in both sections):

```toml
toml = "0.8"
serde = { version = "1", features = ["derive"] }
```

- [ ] **Step 3: Register profile module and export**

In `src/lib.rs`, add:

```rust
pub mod profile;
pub use profile::load_profile;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib profile`
Expected: PASS

- [ ] **Step 5: Wire --profile into main.rs resolve_aircraft()**

Replace the placeholder in `resolve_aircraft()`:

```rust
if let Some(ref path) = args.profile {
    match load_profile(std::path::Path::new(path)) {
        Ok(ac) => ac,
        Err(e) => {
            eprintln!("Error loading profile: {e}");
            process::exit(1);
        }
    }
}
```

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/profile.rs src/lib.rs src/main.rs
git commit -m "feat: add runtime custom profile loading via --profile flag"
```

---

### Task 5: LNM import subcommand

**Files:**
- Create: `src/import.rs`
- Modify: `src/main.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`
- Create: `tests/fixtures/sample.lnmperf` (XML test fixture)
- Create: `tests/fixtures/sample_legacy.lnmperf` (INI test fixture)

- [ ] **Step 1: Add quick-xml dependency**

In `Cargo.toml` under `[dependencies]`:

```toml
quick-xml = { version = "0.37", features = ["serialize"] }
```

- [ ] **Step 2: Create test fixtures**

Create `tests/fixtures/sample.lnmperf` (XML format):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<LittleNavmap>
  <AircraftPerf>
    <Header>
      <CreationDate>2024-01-01T00:00:00</CreationDate>
      <FileVersion>1.0</FileVersion>
      <ProgramName>Little Navmap</ProgramName>
      <ProgramVersion>2.8.0</ProgramVersion>
    </Header>
    <Options>
      <Name>Boeing 737-700</Name>
      <AircraftType>B737</AircraftType>
      <FuelAsVolume>0</FuelAsVolume>
      <JetFuel>1</JetFuel>
    </Options>
    <Perf>
      <MinRunwayLengthFt>5000.000</MinRunwayLengthFt>
      <RunwayType>HARD</RunwayType>
      <UsableFuelLbsGal>46063.000</UsableFuelLbsGal>
      <Climb>
        <FuelFlowLbsGalPerHour>7500.000</FuelFlowLbsGalPerHour>
        <SpeedKtsTAS>276.000</SpeedKtsTAS>
        <VertSpeedFtPerMin>2438.000</VertSpeedFtPerMin>
      </Climb>
      <Cruise>
        <FuelFlowLbsGalPerHour>5200.000</FuelFlowLbsGalPerHour>
        <SpeedKtsTAS>375.000</SpeedKtsTAS>
      </Cruise>
      <Descent>
        <FuelFlowLbsGalPerHour>2800.000</FuelFlowLbsGalPerHour>
        <SpeedKtsTAS>273.000</SpeedKtsTAS>
        <VertSpeedFtPerMin>1389.000</VertSpeedFtPerMin>
      </Descent>
    </Perf>
  </AircraftPerf>
</LittleNavmap>
```

Create `tests/fixtures/sample_legacy.lnmperf` (INI format):

```ini
[Options]
Name=Cessna 152
AircraftType=C152
FuelAsVolume=true
JetFuel=false

[Perf]
UsableFuel=26
TaxiFuelLbsGal=0.13
ClimbVertSpeedFtPerMin=849
ClimbSpeedKtsTAS=63
ClimbFuelFlowLbsGalPerHour=7.0
CruiseSpeedKtsTAS=103
CruiseFuelFlowLbsGalPerHour=6.0
DescentSpeedKtsTAS=84
DescentVertSpeedFtPerMin=591
DescentFuelFlowLbsGalPerHour=3.0
MinRunwayLength=725
RunwayType=Soft
```

- [ ] **Step 3: Create import module structure and implement LNM parser**

The import module uses a directory structure: `src/import/mod.rs` + `src/import/lnmperf.rs`.

Create `src/import/mod.rs`:

```rust
mod lnmperf;

pub use lnmperf::import_lnmperf;

/// Convert lbs to kg
pub(crate) fn lbs_to_kg(lbs: f64) -> f64 {
    lbs * 0.453592
}

/// Convert US gallons to kg using fuel density
pub(crate) fn gal_to_kg(gal: f64, jet_fuel: bool) -> f64 {
    let density_kg_per_gal = if jet_fuel { 3.0390 } else { 2.7216 }; // Jet-A vs Avgas
    gal * density_kg_per_gal
}
```

Create `src/import/lnmperf.rs`:

```rust
use std::path::Path;

use super::{lbs_to_kg, gal_to_kg};

pub struct LnmImportResult {
    pub toml_content: String,
    pub warnings: Vec<String>,
}

pub fn import_lnmperf(path: &Path) -> Result<LnmImportResult, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    if contents.trim_start().starts_with("<?xml") || contents.trim_start().starts_with("<Little") {
        parse_xml(&contents)
    } else {
        parse_ini(&contents)
    }
}

fn parse_xml(contents: &str) -> Result<LnmImportResult, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(contents);
    let mut path: Vec<String> = Vec::new();
    let mut current_text = String::new();

    // Extracted values
    let mut name = String::new();
    let mut icao_type = String::new();
    let mut fuel_as_volume = false;
    let mut jet_fuel = true;
    let mut usable_fuel = 0.0_f64;
    let mut min_runway = 0u32;
    let mut climb_speed = 0.0_f64;
    let mut climb_vs = 0.0_f64;
    let mut climb_ff = 0.0_f64;
    let mut cruise_speed = 0.0_f64;
    let mut cruise_ff = 0.0_f64;
    let mut descent_speed = 0.0_f64;
    let mut descent_vs = 0.0_f64;
    let mut descent_ff = 0.0_f64;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                path.push(String::from_utf8_lossy(e.name().as_ref()).to_string());
                current_text.clear();
            }
            Ok(Event::Text(ref e)) => {
                current_text = e.unescape().unwrap_or_default().trim().to_string();
            }
            Ok(Event::End(_)) => {
                let tag_path = path.join("/");
                match tag_path.as_str() {
                    s if s.ends_with("Options/Name") => name = current_text.clone(),
                    s if s.ends_with("Options/AircraftType") => icao_type = current_text.clone(),
                    s if s.ends_with("Options/FuelAsVolume") => fuel_as_volume = current_text == "1",
                    s if s.ends_with("Options/JetFuel") => jet_fuel = current_text != "0",
                    s if s.ends_with("Perf/UsableFuelLbsGal") => usable_fuel = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Perf/MinRunwayLengthFt") => min_runway = current_text.parse::<f64>().unwrap_or(0.0) as u32,
                    s if s.ends_with("Climb/SpeedKtsTAS") => climb_speed = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Climb/VertSpeedFtPerMin") => climb_vs = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Climb/FuelFlowLbsGalPerHour") => climb_ff = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Cruise/SpeedKtsTAS") => cruise_speed = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Cruise/FuelFlowLbsGalPerHour") => cruise_ff = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Descent/SpeedKtsTAS") => descent_speed = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Descent/VertSpeedFtPerMin") => descent_vs = current_text.parse().unwrap_or(0.0),
                    s if s.ends_with("Descent/FuelFlowLbsGalPerHour") => descent_ff = current_text.parse().unwrap_or(0.0),
                    _ => {}
                }
                path.pop();
                current_text.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("XML parse error: {e}")),
            _ => {}
        }
    }

    build_toml(
        &name, &icao_type, jet_fuel, fuel_as_volume,
        usable_fuel, min_runway,
        climb_speed, climb_vs, climb_ff,
        cruise_speed, cruise_ff,
        descent_speed, descent_vs, descent_ff,
    )
}

fn parse_ini(contents: &str) -> Result<LnmImportResult, String> {
    let mut name = String::new();
    let mut icao_type = String::new();
    let mut fuel_as_volume = false;
    let mut jet_fuel = false;
    let mut usable_fuel = 0.0_f64;
    let mut min_runway = 0u32;
    let mut climb_speed = 0.0_f64;
    let mut climb_vs = 0.0_f64;
    let mut climb_ff = 0.0_f64;
    let mut cruise_speed = 0.0_f64;
    let mut cruise_ff = 0.0_f64;
    let mut descent_speed = 0.0_f64;
    let mut descent_vs = 0.0_f64;
    let mut descent_ff = 0.0_f64;

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Name" => name = value.to_string(),
                "AircraftType" => icao_type = value.to_string(),
                "FuelAsVolume" => fuel_as_volume = value == "true" || value == "1",
                "JetFuel" => jet_fuel = value == "true" || value == "1",
                "UsableFuel" => usable_fuel = value.parse().unwrap_or(0.0),
                "MinRunwayLength" => min_runway = value.parse().unwrap_or(0),
                "ClimbSpeedKtsTAS" => climb_speed = value.parse().unwrap_or(0.0),
                "ClimbVertSpeedFtPerMin" => climb_vs = value.parse().unwrap_or(0.0),
                "ClimbFuelFlowLbsGalPerHour" => climb_ff = value.parse().unwrap_or(0.0),
                "CruiseSpeedKtsTAS" => cruise_speed = value.parse().unwrap_or(0.0),
                "CruiseFuelFlowLbsGalPerHour" => cruise_ff = value.parse().unwrap_or(0.0),
                "DescentSpeedKtsTAS" => descent_speed = value.parse().unwrap_or(0.0),
                "DescentVertSpeedFtPerMin" => descent_vs = value.parse().unwrap_or(0.0),
                "DescentFuelFlowLbsGalPerHour" => descent_ff = value.parse().unwrap_or(0.0),
                _ => {}
            }
        }
    }

    build_toml(
        &name, &icao_type, jet_fuel, fuel_as_volume,
        usable_fuel, min_runway,
        climb_speed, climb_vs, climb_ff,
        cruise_speed, cruise_ff,
        descent_speed, descent_vs, descent_ff,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_toml(
    name: &str, icao_type: &str, jet_fuel: bool, fuel_as_volume: bool,
    usable_fuel: f64, min_runway: u32,
    climb_speed: f64, climb_vs: f64, climb_ff: f64,
    cruise_speed: f64, cruise_ff: f64,
    descent_speed: f64, descent_vs: f64, descent_ff: f64,
) -> Result<LnmImportResult, String> {
    let mut warnings = Vec::new();

    if icao_type.is_empty() {
        return Err("AircraftType is missing".into());
    }

    // Convert fuel to kg
    let convert = |value: f64| -> f64 {
        if fuel_as_volume {
            gal_to_kg(value, jet_fuel)
        } else {
            lbs_to_kg(value)
        }
    };

    let capacity_kg = convert(usable_fuel);
    let climb_ff_kg = convert(climb_ff);
    let cruise_ff_kg = convert(cruise_ff);
    let descent_ff_kg = convert(descent_ff);

    let fuel_type_str = if jet_fuel { "jet" } else { "avgas" };

    // Estimate missing fields
    let cruise_altitude_ft: u32 = if jet_fuel { 36000 } else { 10000 };
    let service_ceiling_ft = cruise_altitude_ft + 5000;
    warnings.push(format!(
        "cruise_altitude_ft estimated as {cruise_altitude_ft} — verify and adjust"
    ));
    warnings.push(format!(
        "service_ceiling_ft estimated as {service_ceiling_ft} — verify and adjust"
    ));

    let toml = format!(
        r#"[aircraft]
name = "{name}"
icao_type = "{icao_type}"

[performance]
cruise_speed_ktas = {cruise_speed:.0}
cruise_altitude_ft = {cruise_altitude_ft}  # ESTIMATED — verify
service_ceiling_ft = {service_ceiling_ft}  # ESTIMATED — verify
min_runway_length_ft = {min_runway}

[performance.climb]
speed_ktas = {climb_speed:.0}
rate_fpm = {climb_vs:.0}

[performance.descent]
speed_ktas = {descent_speed:.0}
rate_fpm = {descent_vs:.0}

[fuel]
capacity_kg = {capacity_kg:.1}
fuel_type = "{fuel_type_str}"

[fuel.flow]
climb_kg_per_hour = {climb_ff_kg:.1}
cruise_kg_per_hour = {cruise_ff_kg:.1}
descent_kg_per_hour = {descent_ff_kg:.1}
"#
    );

    Ok(LnmImportResult { toml_content: toml, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn import_xml_format() {
        let path = Path::new("tests/fixtures/sample.lnmperf");
        let result = import_lnmperf(path).expect("should parse XML");
        assert!(result.toml_content.contains("icao_type = \"B737\""));
        assert!(result.toml_content.contains("cruise_speed_ktas = 375"));
        // Fuel: 46063 lbs -> kg = 46063 * 0.453592 = 20893.6
        assert!(result.toml_content.contains("capacity_kg = 20893"));
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn import_ini_format() {
        let path = Path::new("tests/fixtures/sample_legacy.lnmperf");
        let result = import_lnmperf(path).expect("should parse INI");
        assert!(result.toml_content.contains("icao_type = \"C152\""));
        assert!(result.toml_content.contains("cruise_speed_ktas = 103"));
        // Fuel: 26 gal avgas -> kg = 26 * 2.7216 = 70.8
        assert!(result.toml_content.contains("capacity_kg = 70"));
        assert!(result.toml_content.contains("fuel_type = \"avgas\""));
    }

    #[test]
    fn import_missing_aircraft_type() {
        let ini = "[Options]\nName=Test\n\n[Perf]\nCruiseSpeedKtsTAS=100\n";
        // Write to a temp file
        let dir = std::env::temp_dir();
        let path = dir.join("test_missing_type.lnmperf");
        std::fs::write(&path, ini).unwrap();
        let result = import_lnmperf(&path);
        assert!(result.is_err());
        std::fs::remove_file(&path).ok();
    }
}
```

- [ ] **Step 4: Register import module in lib.rs**

```rust
pub mod import;
pub use import::lnmperf::import_lnmperf;
```

- [ ] **Step 5: Wire import subcommand into main.rs**

Replace the placeholder `import_aircraft()`:

```rust
fn import_aircraft(args: ImportArgs) {
    match args.format.as_str() {
        "lnmperf" => {
            let path = std::path::Path::new(&args.input);
            match import_lnmperf(path) {
                Ok(result) => {
                    // Determine output filename from the generated TOML
                    let output_dir = std::path::Path::new(&args.output);
                    let toml_content = &result.toml_content;

                    // Extract icao_type for filename
                    let icao_type = toml_content
                        .lines()
                        .find(|l| l.starts_with("icao_type"))
                        .and_then(|l| l.split('"').nth(1))
                        .unwrap_or("unknown");
                    let filename = format!("{}.toml", icao_type.to_lowercase());
                    let output_path = output_dir.join(&filename);

                    std::fs::create_dir_all(output_dir).unwrap_or_else(|e| {
                        eprintln!("Error creating output directory: {e}");
                        process::exit(1);
                    });
                    std::fs::write(&output_path, toml_content).unwrap_or_else(|e| {
                        eprintln!("Error writing {}: {e}", output_path.display());
                        process::exit(1);
                    });

                    println!("Imported to {}", output_path.display());
                    for w in &result.warnings {
                        println!("  Warning: {w}");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        other => {
            eprintln!("Error: unsupported format '{other}'. Supported: lnmperf");
            process::exit(1);
        }
    }
}
```

Add the import to main.rs:

```rust
use random_flight::import_lnmperf;
```

- [ ] **Step 6: Run tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/import/ src/lib.rs src/main.rs tests/fixtures/
git commit -m "feat: add LNM import subcommand (aircraft import --format lnmperf)"
```

---

### Task 6: FORMAT.md documentation

**Files:**
- Create: `data/aircraft/FORMAT.md`

- [ ] **Step 1: Write FORMAT.md**

Create `data/aircraft/FORMAT.md` with the complete schema documentation. This should include:

- Purpose and overview
- Complete schema with all fields, types, units, and valid ranges
- Field-by-field descriptions with guidance on sourcing values
- Fully annotated example profile
- Conversion reference (lbs/kg, gal/L, fuel densities)
- Tips for common aircraft categories (GA piston, turboprop, jet)
- Instructions for an AI to produce a valid profile

The document should be self-contained — an AI given only this file and a source of performance data should be able to produce a valid TOML profile.

- [ ] **Step 2: Commit**

```bash
git add data/aircraft/FORMAT.md
git commit -m "docs: add FORMAT.md for aircraft performance TOML schema"
```

---

### Task 7: Expand aircraft database to ~14 profiles

**Files:**
- Create: `data/aircraft/c152.toml`
- Create: `data/aircraft/sr22.toml`
- Create: `data/aircraft/tbm9.toml`
- Create: `data/aircraft/b350.toml`
- Create: `data/aircraft/e190.toml`
- Create: `data/aircraft/b772.toml`
- Create: `data/aircraft/b789.toml`
- Create: `data/aircraft/a333.toml`

Use `FORMAT.md` and public performance data to create accurate profiles. Source data from LNM community files where available, cross-reference with manufacturer specs.

- [ ] **Step 1: Create the 8 new TOML profiles**

Write each profile using realistic performance data. Validate that the values are within the sane ranges specified in FORMAT.md.

- [ ] **Step 2: Build and test**

Run: `cargo build`
Expected: PASS — build.rs should pick up all 14 TOML files

Run: `cargo test`
Expected: PASS

- [ ] **Step 3: Verify aircraft list**

Run: `cargo run -- aircraft list`
Expected: shows all 14 aircraft with reasonable values

- [ ] **Step 4: Commit**

```bash
git add data/aircraft/
git commit -m "feat: expand aircraft database to 14 profiles"
```

---

### Task 8: Update CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update command examples**

Update the example commands to reflect the new CLI structure:

```bash
cargo run -- generate --aircraft B738 --time 4h
cargo run -- generate --profile custom.toml --time 3h
cargo run -- aircraft list
cargo run -- aircraft import --format lnmperf input.lnmperf
```

- [ ] **Step 2: Update architecture section**

Add description of the TOML data pipeline and the import module.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: update CLAUDE.md for new aircraft data pipeline and CLI"
```

---

### Task 9: Final validation

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy`
Expected: no warnings

- [ ] **Step 3: Test CLI end-to-end**

Run: `cargo run -- generate --aircraft B738 --time 4h`
Run: `cargo run -- generate --aircraft C152 --time 1h`
Run: `cargo run -- aircraft list`
Run: `cargo run -- aircraft import --format lnmperf tests/fixtures/sample.lnmperf --output /tmp/`

Verify all produce expected output.

- [ ] **Step 4: Verify round-trip for a profile**

Run: `cargo run -- aircraft import --format lnmperf tests/fixtures/sample.lnmperf --output /tmp/`
Then: `cargo run -- generate --profile /tmp/b737.toml --time 3h`
Expected: successfully generates a flight plan using the imported profile.
