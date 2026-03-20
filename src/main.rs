use std::process;
use std::time::Duration;

use clap::{Parser, Subcommand};
use clap::builder::styling::{AnsiColor, Effects, Styles};

use random_flight::{
    Aircraft, FlightPlanOptions, aircraft_by_name, built_in_aircraft,
    generate_flight_plan,
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
        random-flight aircraft",
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
            random-flight generate --aircraft custom --cruise-speed 450 --cruise-alt 35000 \
            --climb-rate 2000 --descent-rate 1800 --range 3000 --min-runway 6000 --time 3h"
    )]
    Generate(GenerateArgs),
    /// List available aircraft presets
    Aircraft,
}

#[derive(Parser)]
struct GenerateArgs {
    /// Aircraft preset name (e.g. C172, B738, A320) or "custom"
    #[arg(long)]
    aircraft: String,

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

    /// Cruise speed in knots
    #[arg(long, help_heading = "Custom Aircraft")]
    cruise_speed: Option<u16>,

    /// Cruise altitude in feet
    #[arg(long, help_heading = "Custom Aircraft")]
    cruise_alt: Option<u32>,

    /// Climb rate in ft/min
    #[arg(long, help_heading = "Custom Aircraft")]
    climb_rate: Option<u16>,

    /// Descent rate in ft/min
    #[arg(long, help_heading = "Custom Aircraft")]
    descent_rate: Option<u16>,

    /// Range in nautical miles
    #[arg(long, help_heading = "Custom Aircraft")]
    range: Option<u32>,

    /// Minimum runway length in feet
    #[arg(long, help_heading = "Custom Aircraft")]
    min_runway: Option<u32>,
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
        Commands::Aircraft => list_aircraft(),
        Commands::Generate(args) => generate(args),
    }
}

fn list_aircraft() {
    println!("Available aircraft presets:\n");
    println!("  {:<6} {:>5}  {:>7}  {:>8}  {:>10}",
        "NAME", "SPD", "ALT", "RANGE", "MIN RWY");
    println!("  {:<6} {:>5}  {:>7}  {:>8}  {:>10}",
        "----", "---", "---", "-----", "-------");
    for a in built_in_aircraft() {
        println!("  {:<6} {:>3} kt  FL{:03}    {:>5} nm  {:>6} ft",
            a.name, a.cruise_speed_kts, a.cruise_altitude_ft / 100,
            a.range_nm, a.min_runway_length_ft);
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
            println!("Aircraft:    {}", fp.aircraft.name);
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
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn resolve_aircraft(args: &GenerateArgs) -> Aircraft {
    if args.aircraft.eq_ignore_ascii_case("custom") {
        let missing = |field: &str| -> ! {
            eprintln!("Error: --{field} is required for custom aircraft");
            process::exit(1);
        };
        Aircraft {
            name: "Custom",
            cruise_speed_kts: args.cruise_speed.unwrap_or_else(|| missing("cruise-speed")),
            cruise_altitude_ft: args.cruise_alt.unwrap_or_else(|| missing("cruise-alt")),
            climb_rate_fpm: args.climb_rate.unwrap_or_else(|| missing("climb-rate")),
            descent_rate_fpm: args.descent_rate.unwrap_or_else(|| missing("descent-rate")),
            climb_speed_factor: 0.75,
            descent_speed_factor: 0.65,
            range_nm: args.range.unwrap_or_else(|| missing("range")),
            min_runway_length_ft: args.min_runway.unwrap_or_else(|| missing("min-runway")),
        }
    } else {
        match aircraft_by_name(&args.aircraft) {
            Some(a) => a.clone(),
            None => {
                eprintln!("Error: unknown aircraft '{}'. Run `random-flight aircraft` to see presets.", args.aircraft);
                process::exit(1);
            }
        }
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
