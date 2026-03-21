use std::process;
use std::time::Duration;

use clap::{Parser, Subcommand};
use clap::builder::styling::{AnsiColor, Effects, Styles};

use random_flight::{
    Aircraft, FlightPlanOptions, aircraft_by_icao_type, built_in_aircraft,
    generate_flight_plan, import_lnmperf, load_profile,
};

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser)]
#[command(
    name = "random-flight",
    about = "Generate random flight plans for flight simulators",
    long_about = "Generate random flight plans for flight simulators.\n\n\
        Pick an aircraft, set a target block time, and get a realistic departure/arrival \
        pair with full climb/cruise/descent breakdown.",
    subcommand_required = true,
    styles = STYLES,
    after_help = "Examples:\n  \
        random-flight generate --aircraft B738 --time 4h\n  \
        random-flight generate --aircraft C172 --time 1h30m --departure KJFK\n  \
        random-flight aircraft list",
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

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

fn main() {
    // Print help and exit 0 when invoked with no arguments.
    // clap's arg_required_else_help exits with code 2, so we handle this manually.
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

fn import_aircraft(args: ImportArgs) {
    match args.format.as_str() {
        "lnmperf" => {}
        other => {
            eprintln!("Error: unsupported format '{other}'. Supported: lnmperf");
            process::exit(1);
        }
    }

    let path = std::path::Path::new(&args.input);
    let result = match import_lnmperf(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    let output_dir = std::path::Path::new(&args.output);
    let output_path = output_dir.join(format!("{}.toml", result.icao_type.to_lowercase()));

    if let Err(e) = std::fs::write(&output_path, &result.toml_content) {
        eprintln!("Error writing {}: {e}", output_path.display());
        process::exit(1);
    }

    println!("Wrote {}", output_path.display());

    for warning in &result.warnings {
        println!("  warning: {warning}");
    }
}

fn generate(args: GenerateArgs) {
    let aircraft = resolve_aircraft(&args);
    let target = parse_duration(&args.time);
    let tolerance = parse_duration(&args.tolerance);

    let opts = FlightPlanOptions {
        tolerance,
        departure_icao: args.departure,
        arrival_icao: args.arrival,
        ..Default::default()
    };

    match generate_flight_plan(&aircraft, target, Some(opts)) {
        Ok(fp) => {
            println!("Flight Plan");
            println!("===========");
            println!("Aircraft:    {} ({})", fp.aircraft.icao_type, fp.aircraft.name);
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
            println!();
            println!("SimBrief:    {}", fp.simbrief_url());
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn resolve_aircraft(args: &GenerateArgs) -> Aircraft {
    if let Some(ref path) = args.profile {
        match load_profile(std::path::Path::new(path)) {
            Ok(ac) => ac,
            Err(e) => {
                eprintln!("Error loading profile: {e}");
                process::exit(1);
            }
        }
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
