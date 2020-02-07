use std::fs;
use regex::Regex;
use std::time::Duration;
use std::str::FromStr;

const PSEUDO_FS_PATH: &str = "/sys/class/power_supply/";
const TLP_THRESHOLD_PERCENTAGE: f32 = 1.0;

/// Battery status enum. 'Passive' denotes the 'Unknown' state provided by sysfs
/// when TLP enforces a threshold
enum Status {
    Charging,
    Discharging,
    Passive,
}

/// A configuration of batteries on a given machine
struct Configuration {
    time_to_completion: Duration,
    percentage: f32,
    status: Status,
}

/// A battery and all its concomitant data. Note that units are as-is, provided by sysfs in millis
struct Battery {
    status: Status,
    // Unit: mWh
    current_charge: u32,
    // Unit: mWh
    max_charge: u32,
    // Unit: mW
    power_draw: u32,
}

fn main() {
    let config = get_configuration();
    print_status(config);
}

/// Print a formatted status-line string
fn print_status(config: Configuration) {
    // Print percentage as an actual percentage and calculate pretty display-time
    println!("{:.2}%{}", config.percentage * 100 as f32, calc_display_time(config.status, config.time_to_completion));
}

/// Calculate display-time and format display-string according to status
fn calc_display_time(status: Status, time: Duration) -> String {
    // Calculate hours and minutes for printing
    let hours = time.as_secs() / 3600;
    let minutes = time.as_secs() % 3600 / 60;
    // Match on status and format string accordingly with {+, -}, printing empty when irrelevant
    match status {
        Status::Charging => {
            format!(" (+{}:{:02})", hours, minutes)
        }
        Status::Discharging => {
            format!(" (-{}:{:02})", hours, minutes)
        }
        Status::Passive => { "".to_string() }
    }
}

/// Find, calculate, and return a configuration of batteries and its values
fn get_configuration() -> Configuration {
    // Matches any number of batteries on sysfs
    let regex = Regex::new(r"^BAT\d+$").unwrap();

    // Temporary vector for holding discovered batteries
    let mut batteries: Vec<Battery> = Vec::new();

    // Read 'power_supply' dir on sysfs
    let paths = fs::read_dir(PSEUDO_FS_PATH).unwrap();

    // For each result, match on batteries, and dispatch getters
    // for Battery-struct creation before pushing onto vector
    for path in paths {
        if let Ok(e) = path {
            if regex.is_match(e.file_name().to_str().unwrap()) {
                let battery_name: String = e.file_name().to_str().unwrap().parse().unwrap();
                batteries.push(Battery {
                    current_charge: get_current_charge(&battery_name),
                    max_charge: get_max_charge(&battery_name),
                    status: get_status(&battery_name),
                    power_draw: get_power_draw(&battery_name),
                });
            }
        }
    }
    // Find status of all batteries.
    // Assumes that all batteries will be either charging or discharging, if not passive
    let mut stat = Status::Passive;
    for bat in &batteries {
        match bat.status {
            Status::Charging => {
                stat = Status::Charging;
                break;
            }
            Status::Discharging => {
                stat = Status::Discharging;
                break;
            }
            _ => {}
        }
    }

    // Create configuration, calculating both time-to-completion and percentage.
    let configuration = Configuration {
        time_to_completion: calc_time(&batteries, &stat),
        percentage: calc_percentage(&batteries),
        status: stat,
    };
    return configuration;
}

/// Calculate time-to-completion based on current values
fn calc_time(bats: &Vec<Battery>, stat: &Status) -> Duration {
    let total_current_charge: u32 = bats.iter().map(|x| x.current_charge).sum();
    let total_max_charge: u32 = bats.iter().map(|x| x.max_charge).sum();
    let total_draw: u32 = bats.iter().map(|x| x.power_draw).sum();
    match stat {
        Status::Passive => {
            Duration::new(0, 0)
        }
        Status::Discharging => {
            Duration::new((((total_current_charge as f32) / (total_draw as f32)) * 3600f32) as u64, 0)
        }
        Status::Charging => {
            Duration::new(((((total_max_charge as f32 * TLP_THRESHOLD_PERCENTAGE) - total_current_charge as f32)
                / (total_draw as f32)) * 3600f32) as u64, 0)
        }
    }
}

/// Calculate charge-percentage across all batteries
fn calc_percentage(bats: &Vec<Battery>) -> f32 {
    let total_charge: u32 = bats.iter().map(|x| x.max_charge).sum();
    let total_current_charge: u32 = bats.iter().map(|x| x.current_charge).sum();

    return (total_current_charge as f32) / (total_charge as f32);
}

/// Return current charge of given battery
fn get_current_charge(bat: &String) -> u32 {
    let cap = fs::read_to_string(format!("{}{}/energy_now", PSEUDO_FS_PATH, bat)).unwrap();
    return u32::from_str(cap.trim()).unwrap();
}

/// Return max charge of given battery
fn get_max_charge(bat: &String) -> u32 {
    let cap = fs::read_to_string(format!("{}{}/energy_full", PSEUDO_FS_PATH, bat)).unwrap();
    return u32::from_str(cap.trim()).unwrap();
}

/// Return current power draw of given battery
fn get_power_draw(bat: &String) -> u32 {
    let power_draw = fs::read_to_string(format!("{}{}/power_now", PSEUDO_FS_PATH, bat)).unwrap();
    return u32::from_str(power_draw.trim()).unwrap();
}

/// Return current status of given battery
fn get_status(bat: &String) -> Status {
    let raw_status = fs::read_to_string(format!("{}{}/status", PSEUDO_FS_PATH, bat)).unwrap();
    let stat = raw_status.trim();
    match stat {
        "Unknown" => { Status::Passive }
        "Full" => {Status::Passive}
        "Charging" => { Status::Charging }
        "Discharging" => { Status::Discharging }
        _ => {
            panic!("Could not match status of battery: {}, status received was: {}", bat, stat);
        }
    }
}

