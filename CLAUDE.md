# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Random-flight is a Rust CLI tool that generates random flight plans for flight simulators. It selects realistic departure/arrival airports from an embedded database, calculates great-circle distances, and estimates block times using a climb/cruise/descent flight model.

## Commands

```bash
cargo build              # Build (triggers build.rs which generates airport DB)
cargo test               # Run all tests (unit + integration)
cargo test --lib         # Unit tests only
cargo test --test integration  # Integration tests only
cargo test <test_name>   # Run a single test by name
cargo clippy             # Lint
cargo run -- generate --aircraft B738 --time 4h   # Example run
cargo run -- aircraft                              # List aircraft presets
```

## Architecture

**Build-time data pipeline** (`build.rs`): Downloads airport/runway CSVs from OurAirports, filters to hard-surface runways and ICAO-coded airports, and generates `airport_db.rs` compiled into the binary. Falls back to cached CSVs in `data/` if download fails.

**Core flow**: CLI (`src/main.rs`) → `selection::generate_flight_plan()` → `flight_plan::calculate_flight_plan()` → `geo::haversine_distance_nm()`

**Selection algorithm** (`selection.rs`): Estimates target distance from block time, filters airports by runway length, then randomly samples departure/arrival pairs within a distance band until one matches the target block time within tolerance. Supports pinning departure and/or arrival.

**Flight model** (`flight_plan.rs`): Computes climb/cruise/descent phases based on aircraft performance. Automatically reduces cruise altitude for short routes where climb+descent distance exceeds total distance.

**Key design choices**:
- Airport/aircraft data use `'static` references (compile-time embedded)
- RNG is injectable for deterministic testing
- `thiserror` for error types, `clap` derive for CLI parsing
- `humantime` for duration parsing (e.g., "2h30m", "90m")
