use std::time::Duration;

use rand::SeedableRng;
use rand::rngs::SmallRng;

use random_flight::{
    FlightPlanOptions, aircraft_by_icao_type, generate_flight_plan_with_rng,
};

#[test]
fn c172_one_hour_flight() {
    let ac = aircraft_by_icao_type("C172").unwrap();
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
    let ac = aircraft_by_icao_type("B738").unwrap();
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
    let ac = aircraft_by_icao_type("B738").unwrap();
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
    let ac = aircraft_by_icao_type("A320").unwrap();
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

#[test]
fn cli_generate_produces_flight_plan() {
    let bin = env!("CARGO_BIN_EXE_random-flight");
    let output = std::process::Command::new(bin)
        .args(["generate", "C172", "1h"])
        .output()
        .expect("failed to run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Flight Plan"), "expected flight plan output, got: {stdout}");
    assert!(stdout.contains("Departure:"));
    assert!(stdout.contains("Arrival:"));
    assert!(stdout.contains("SimBrief:    https://dispatch.simbrief.com/options/custom?"),
        "expected SimBrief URL in output, got: {stdout}");
    assert!(stdout.contains("type=C172"), "expected type=C172 in SimBrief URL, got: {stdout}");
}

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

#[test]
fn cli_generate_with_profile() {
    let bin = env!("CARGO_BIN_EXE_random-flight");
    let output = std::process::Command::new(bin)
        .args(["generate", "--profile", "data/aircraft/b738.toml", "4h"])
        .output()
        .expect("failed to run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("B738"), "expected B738 in output, got: {stdout}");
    assert!(stdout.contains("Flight Plan"), "expected flight plan output");
    assert!(stdout.contains("SimBrief:    https://dispatch.simbrief.com/options/custom?"),
        "expected SimBrief URL in profile output, got: {stdout}");
}

#[test]
fn cli_aircraft_import_lnmperf() {
    let bin = env!("CARGO_BIN_EXE_random-flight");
    let output = std::process::Command::new(bin)
        .args(["aircraft", "import", "lnmperf",
               "tests/fixtures/sample.lnmperf", "--output", "/tmp/claude-1000/"])
        .output()
        .expect("failed to run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("b737.toml"), "expected output filename in output, got: {stdout}");
}

#[test]
fn cli_no_subcommand_shows_help() {
    let bin = env!("CARGO_BIN_EXE_random-flight");
    let output = std::process::Command::new(bin)
        .output()
        .expect("failed to run binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Manual help guard in main() prints to stdout and exits 0
    assert!(output.status.success());
    assert!(stdout.contains("generate"), "expected 'generate' in help output");
    assert!(stdout.contains("aircraft"), "expected 'aircraft' in help output");
}
