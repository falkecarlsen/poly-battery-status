use std::fs;
use regex::Regex;
use std::time::Duration;
use std::str::FromStr;
use std::fmt::Debug;

const PSEUDO_FS_PATH: &str = "/sys/class/power_supply/";

enum Status {
    Unknown,
    Charging,
    Discharging,
}

struct Configuration {
    batteries: Vec<Battery>,
    time_to_fin: Duration,
    percentage: f32,
    status: Status,
}

struct Battery {
    name: String,
    // Unit: mWh
    current_charge: u32,
    max_charge: u32,
    status: Status,
    // Unit: mW
    power_draw: u32,
}

fn main() {
    let configuration = get_configuration();
    let direction = match configuration.status {
        Status::Charging => { "+" }
        Status::Discharging => { "-" }
        Status::Unknown => { "" }
    };
    let hours = configuration.time_to_fin.as_secs() / 3600;
    let mins = configuration.time_to_fin.as_secs() % 3600 / 60;
    println!("{:.2}% ({}{}:{:02})", configuration.percentage * 100 as f32, direction, hours, mins);
}

fn get_configuration() -> Configuration {
    let regex = Regex::new(r"^BAT\d+$").unwrap();

    let mut batteries: Vec<Battery> = Vec::new();

    let paths = fs::read_dir(PSEUDO_FS_PATH).unwrap();

    for path in paths {
        if let Ok(e) = path {
            if regex.is_match(e.file_name().to_str().unwrap()) {
                let battery_name: String = e.file_name().to_str().unwrap().parse().unwrap();
                batteries.push(Battery {
                    current_charge: get_current_charge(&battery_name),
                    max_charge: get_max_charge(&battery_name),
                    status: get_status(&battery_name),
                    power_draw: get_power_draw(&battery_name),
                    name: battery_name,
                });
            }
        }
    }
    let mut stat = Status::Unknown;
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

    let configuration = Configuration {
        time_to_fin: calc_time(&batteries, &stat),
        percentage: calc_percentage(&batteries),
        batteries,
        status: stat,
    };
    return configuration;
}

fn calc_time(bats: &Vec<Battery>, stat: &Status) -> Duration {
    let total_cap: u32 = bats.iter().map(|x| x.current_charge).sum();
    let total_draw: u32 = bats.iter().map(|x| x.power_draw).sum();
    match stat {
        Status::Unknown => {
            Duration::new(0, 0)
        }
        _ => {
            Duration::new((((total_cap as f32) / (total_draw as f32)) * 3600f32) as u64, 0)
        }
    }
}

fn calc_percentage(bats: &Vec<Battery>) -> f32 {
    let total_charge: u32 = bats.iter().map(|x| x.max_charge).sum();
    let total_current_charge: u32 = bats.iter().map(|x| x.current_charge).sum();

    return (total_current_charge as f32) / (total_charge as f32);
}

fn get_current_charge(bat: &String) -> u32 {
    let cap = fs::read_to_string(format!("{}{}/energy_now", PSEUDO_FS_PATH, bat)).unwrap();
    return u32::from_str(cap.trim()).unwrap();
}

fn get_max_charge(bat: &String) -> u32 {
    let cap = fs::read_to_string(format!("{}{}/energy_full", PSEUDO_FS_PATH, bat)).unwrap();
    return u32::from_str(cap.trim()).unwrap();
}

fn get_status(bat: &String) -> Status {
    let raw_status = fs::read_to_string(format!("{}{}/status", PSEUDO_FS_PATH, bat)).unwrap();
    let stat = raw_status.trim();
    match stat {
        "Unknown" => { Status::Unknown }
        "Charging" => { Status::Charging }
        "Discharging" => { Status::Discharging }
        _ => {
            panic!("Could not match status of battery: {}, stat was: {}", bat, stat);
        }
    }
}

fn get_power_draw(bat: &String) -> u32 {
    let power_draw = fs::read_to_string(format!("{}{}/power_now", PSEUDO_FS_PATH, bat)).unwrap();
    return u32::from_str(power_draw.trim()).unwrap();
}
