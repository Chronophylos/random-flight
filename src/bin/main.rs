use std::process;
use std::time::Duration;

use clap::Parser;

use random_flight::{
    Aircraft, FlightPlanOptions, aircraft_by_name, built_in_aircraft,
    generate_flight_plan,
};

#[derive(Parser)]
#[command(name = "random-flight", about = "Generate random flight plans for flight simulators")]
struct Cli {
    /// Aircraft preset name (e.g. C172, B738, A320) or "custom"
    #[arg(long, required_unless_present = "list_aircraft")]
    aircraft: Option<String>,

    /// Target block time (e.g. 2h, 2h30m, 90m)
    #[arg(long, required_unless_present = "list_aircraft")]
    time: Option<String>,

    /// Tolerance around target time (default: 15m)
    #[arg(long, default_value = "15m")]
    tolerance: String,

    /// Pin departure airport (ICAO code)
    #[arg(long)]
    departure: Option<String>,

    /// Pin arrival airport (ICAO code)
    #[arg(long)]
    arrival: Option<String>,

    /// Custom aircraft: cruise speed in knots
    #[arg(long)]
    cruise_speed: Option<u16>,

    /// Custom aircraft: cruise altitude in feet
    #[arg(long)]
    cruise_alt: Option<u32>,

    /// Custom aircraft: climb rate in ft/min
    #[arg(long)]
    climb_rate: Option<u16>,

    /// Custom aircraft: descent rate in ft/min
    #[arg(long)]
    descent_rate: Option<u16>,

    /// Custom aircraft: range in nautical miles
    #[arg(long)]
    range: Option<u32>,

    /// Custom aircraft: minimum runway length in feet
    #[arg(long)]
    min_runway: Option<u32>,

    /// List available aircraft presets
    #[arg(long)]
    list_aircraft: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.list_aircraft {
        println!("Available aircraft presets:");
        for a in built_in_aircraft() {
            println!("  {:<6} {} kts, FL{:03}, range {} nm, min rwy {} ft",
                a.name, a.cruise_speed_kts, a.cruise_altitude_ft / 100,
                a.range_nm, a.min_runway_length_ft);
        }
        return;
    }

    let aircraft_name = cli.aircraft.as_deref().unwrap();
    let aircraft = if aircraft_name.eq_ignore_ascii_case("custom") {
        let missing = |field: &str| -> ! {
            eprintln!("Error: --{field} is required for custom aircraft");
            process::exit(1);
        };
        Aircraft {
            name: "Custom",
            cruise_speed_kts: cli.cruise_speed.unwrap_or_else(|| missing("cruise-speed")),
            cruise_altitude_ft: cli.cruise_alt.unwrap_or_else(|| missing("cruise-alt")),
            climb_rate_fpm: cli.climb_rate.unwrap_or_else(|| missing("climb-rate")),
            descent_rate_fpm: cli.descent_rate.unwrap_or_else(|| missing("descent-rate")),
            climb_speed_factor: 0.75,
            descent_speed_factor: 0.65,
            range_nm: cli.range.unwrap_or_else(|| missing("range")),
            min_runway_length_ft: cli.min_runway.unwrap_or_else(|| missing("min-runway")),
        }
    } else {
        match aircraft_by_name(aircraft_name) {
            Some(a) => a.clone(),
            None => {
                eprintln!("Error: unknown aircraft '{}'. Use --list-aircraft to see presets.", aircraft_name);
                process::exit(1);
            }
        }
    };

    let target = parse_duration(cli.time.as_deref().unwrap());
    let tolerance = parse_duration(&cli.tolerance);

    let opts = FlightPlanOptions {
        tolerance,
        departure_icao: cli.departure,
        arrival_icao: cli.arrival,
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
