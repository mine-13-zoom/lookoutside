use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    thread,
};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_lookoutside"))
}

#[test]
fn prints_version() {
    let output = cli().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "lookoutside 0.1.0"
    );
}

#[test]
fn rejects_invalid_coordinates_before_networking() {
    let output = cli().args(["--lat", "91", "--lon", "0"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr)
        .contains("latitude must be between -90 and 90 degrees"));
}

#[test]
fn requires_both_coordinates() {
    let output = cli().args(["--lat", "42"]).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--lon <DEGREES>"));
}

#[test]
fn resolves_a_place_and_prints_json() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (stream, _) = listener.accept().unwrap();
            respond(stream);
        }
    });

    let base = format!("http://{address}");
    let output = cli()
        .args(["Testville", "--json"])
        .env("LOOKOUTSIDE_GEOCODING_URL", format!("{base}/geocode"))
        .env("LOOKOUTSIDE_FORECAST_URL", format!("{base}/forecast"))
        .output()
        .unwrap();

    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["location"]["name"], "Testville, Test Region, Testland");
    assert_eq!(json["location"]["source"], "search");
    assert_eq!(json["condition"], "Partly cloudy");
    assert_eq!(json["current"]["temperature_2m"], 18.5);
    assert_eq!(json["today"]["temperature_max"], 22.0);
}

fn respond(mut stream: TcpStream) {
    let mut request = [0_u8; 4096];
    let size = stream.read(&mut request).unwrap();
    let request = String::from_utf8_lossy(&request[..size]);

    let body = if request.starts_with("GET /geocode?") {
        r#"{"results":[{"name":"Testville","latitude":52.0,"longitude":13.0,"admin1":"Test Region","country":"Testland","country_code":"TL"}]}"#
    } else if request.starts_with("GET /forecast?") {
        r#"{
            "timezone":"Europe/Berlin",
            "timezone_abbreviation":"CEST",
            "current":{
                "time":"2026-07-18T15:00",
                "temperature_2m":18.5,
                "relative_humidity_2m":61.0,
                "apparent_temperature":18.0,
                "is_day":1,
                "precipitation":0.0,
                "weather_code":2,
                "cloud_cover":42.0,
                "pressure_msl":1015.0,
                "wind_speed_10m":9.5,
                "wind_direction_10m":225.0,
                "wind_gusts_10m":18.0
            },
            "current_units":{
                "temperature_2m":"°C",
                "relative_humidity_2m":"%",
                "apparent_temperature":"°C",
                "precipitation":"mm",
                "cloud_cover":"%",
                "pressure_msl":"hPa",
                "wind_speed_10m":"km/h",
                "wind_direction_10m":"°",
                "wind_gusts_10m":"km/h"
            },
            "daily":{
                "temperature_2m_max":[22.0],
                "temperature_2m_min":[12.0],
                "precipitation_probability_max":[20.0],
                "sunrise":["2026-07-18T05:00"],
                "sunset":["2026-07-18T21:00"]
            },
            "daily_units":{
                "temperature_2m_max":"°C",
                "temperature_2m_min":"°C",
                "precipitation_probability_max":"%"
            }
        }"#
    } else {
        panic!("unexpected request: {request}");
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes()).unwrap();
}
