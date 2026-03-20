use csv::ReaderBuilder;
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const AIRPORTS_URL: &str = "https://davidmegginson.github.io/ourairports-data/airports.csv";
const RUNWAYS_URL: &str = "https://davidmegginson.github.io/ourairports-data/runways.csv";

#[derive(Debug, Deserialize)]
struct RawAirport {
    ident: String,
    #[serde(rename = "type")]
    airport_type: String,
    name: String,
    latitude_deg: f64,
    longitude_deg: f64,
    elevation_ft: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRunway {
    airport_ident: String,
    length_ft: Option<String>,
    surface: Option<String>,
    closed: Option<u8>,
}

fn download_or_fallback(url: &str, fallback_path: &Path, dest_path: &Path) {
    if let Ok(body) = try_download(url) {
        fs::write(dest_path, &body).expect("failed to write downloaded CSV");
        // Also update fallback
        fs::create_dir_all(fallback_path.parent().unwrap()).ok();
        fs::write(fallback_path, &body).ok();
    } else {
        eprintln!("cargo:warning=Failed to download {url}, using fallback");
        if fallback_path.exists() {
            fs::copy(fallback_path, dest_path).expect("fallback copy failed");
        } else {
            panic!("No fallback data at {} and download failed", fallback_path.display());
        }
    }
}

fn try_download(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let response = ureq::get(url).call()?;
    let body = response.into_body().read_to_vec()?;
    Ok(body)
}

fn is_hard_surface(surface: &str) -> bool {
    let s = surface.to_uppercase();
    s.starts_with("ASP") || s.starts_with("CON") || s.starts_with("BIT")
        || s.starts_with("PEM") || s == "ASPHALT" || s == "CONCRETE"
        || s == "BITUMINOUS" || s.starts_with("ASPH") || s.starts_with("CONC")
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let data_dir = manifest_dir.join("data");

    let airports_tmp = out_dir.join("airports.csv");
    let runways_tmp = out_dir.join("runways.csv");

    download_or_fallback(AIRPORTS_URL, &data_dir.join("airports.csv"), &airports_tmp);
    download_or_fallback(RUNWAYS_URL, &data_dir.join("runways.csv"), &runways_tmp);

    // Parse runways: map airport_ident -> longest hard-surface non-closed runway length
    let mut best_runway: HashMap<String, u32> = HashMap::new();
    {
        let mut rdr = ReaderBuilder::new().from_path(&runways_tmp).expect("open runways.csv");
        for result in rdr.deserialize::<RawRunway>() {
            let rwy = match result {
                Ok(r) => r,
                Err(_) => continue,
            };
            if rwy.closed == Some(1) {
                continue;
            }
            if !matches!(&rwy.surface, Some(s) if is_hard_surface(s)) {
                continue;
            }
            let length: u32 = match &rwy.length_ft {
                Some(s) => match s.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                },
                None => continue,
            };
            let entry = best_runway.entry(rwy.airport_ident.clone()).or_insert(0);
            if length > *entry {
                *entry = length;
            }
        }
    }

    // Parse airports, filter, and generate code
    let mut airports = Vec::new();
    {
        let mut rdr = ReaderBuilder::new().from_path(&airports_tmp).expect("open airports.csv");
        for result in rdr.deserialize::<RawAirport>() {
            let apt = match result {
                Ok(a) => a,
                Err(_) => continue,
            };
            // Skip non-airports
            match apt.airport_type.as_str() {
                "large_airport" | "medium_airport" | "small_airport" => {}
                _ => continue,
            }
            // Must have ICAO-style ident (4 letters)
            if apt.ident.len() != 4 || !apt.ident.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }
            // Must have a hard-surface runway
            let runway_len = match best_runway.get(&apt.ident) {
                Some(&len) => len,
                None => continue,
            };
            let elevation: i32 = apt
                .elevation_ft
                .as_deref()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

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
            ));
        }
    }

    // Generate Rust source
    let gen_path = out_dir.join("airport_db.rs");
    let mut f = fs::File::create(&gen_path).expect("create airport_db.rs");

    writeln!(f, "static AIRPORTS: &[Airport] = &[").unwrap();
    for (icao, name, lat, lon, elev, rwy) in &airports {
        writeln!(
            f,
            "    Airport {{ icao: \"{icao}\", name: \"{name}\", latitude: {lat}_f64, longitude: {lon}_f64, elevation_ft: {elev}, runway_length_ft: {rwy} }},"
        )
        .unwrap();
    }
    writeln!(f, "];").unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=data/airports.csv");
    println!("cargo:rerun-if-changed=data/runways.csv");
    println!("cargo:warning=Generated airport DB with {} airports", airports.len());
}
