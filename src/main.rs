use std::{env, io::IsTerminal, time::Duration};

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const IP_LOCATION_URL: &str = "https://ipwho.is/";
const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";

#[derive(Debug, Parser)]
#[command(
    name = "lookoutside",
    version,
    about = "Look outside from your terminal with current local weather",
    after_help = "Examples:\n  lookoutside\n  lookoutside London\n  lookoutside \"New York\" --units imperial\n  lookoutside --lat 59.33 --lon 18.07"
)]
struct Args {
    /// A city, postal code, or place name. Your approximate IP location is used when omitted.
    #[arg(value_name = "PLACE", conflicts_with_all = ["latitude", "longitude"])]
    place: Option<String>,

    /// Latitude to use directly.
    #[arg(
        long = "lat",
        value_name = "DEGREES",
        requires = "longitude",
        allow_hyphen_values = true
    )]
    latitude: Option<f64>,

    /// Longitude to use directly.
    #[arg(
        long = "lon",
        value_name = "DEGREES",
        requires = "latitude",
        allow_hyphen_values = true
    )]
    longitude: Option<f64>,

    /// Unit system for temperatures, wind, and precipitation.
    #[arg(short, long, value_enum, default_value_t = Units::Metric)]
    units: Units,

    /// Print machine-readable JSON instead of the weather card.
    #[arg(long)]
    json: bool,

    /// Disable ANSI colors in terminal output.
    #[arg(long)]
    no_color: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum Units {
    #[default]
    Metric,
    Imperial,
}

impl Units {
    fn temperature_api_value(self) -> &'static str {
        match self {
            Self::Metric => "celsius",
            Self::Imperial => "fahrenheit",
        }
    }

    fn wind_api_value(self) -> &'static str {
        match self {
            Self::Metric => "kmh",
            Self::Imperial => "mph",
        }
    }

    fn precipitation_api_value(self) -> &'static str {
        match self {
            Self::Metric => "mm",
            Self::Imperial => "inch",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Location {
    name: String,
    latitude: f64,
    longitude: f64,
    source: &'static str,
}

#[derive(Debug, Deserialize)]
struct IpLocationResponse {
    success: bool,
    message: Option<String>,
    city: Option<String>,
    region: Option<String>,
    country: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    flag: Option<IpFlag>,
}

#[derive(Debug, Deserialize)]
struct IpFlag {
    emoji: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeocodingResult>>,
}

#[derive(Debug, Deserialize)]
struct GeocodingResult {
    name: String,
    latitude: f64,
    longitude: f64,
    admin1: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WeatherResponse {
    timezone: String,
    timezone_abbreviation: String,
    current: CurrentWeather,
    current_units: CurrentUnits,
    daily: DailyWeather,
    daily_units: DailyUnits,
}

#[derive(Debug, Deserialize, Serialize)]
struct CurrentWeather {
    time: String,
    temperature_2m: f64,
    relative_humidity_2m: f64,
    apparent_temperature: f64,
    is_day: u8,
    precipitation: f64,
    weather_code: u16,
    cloud_cover: f64,
    pressure_msl: f64,
    wind_speed_10m: f64,
    wind_direction_10m: f64,
    wind_gusts_10m: f64,
}

#[derive(Debug, Deserialize, Serialize)]
struct CurrentUnits {
    temperature_2m: String,
    relative_humidity_2m: String,
    apparent_temperature: String,
    precipitation: String,
    cloud_cover: String,
    pressure_msl: String,
    wind_speed_10m: String,
    wind_direction_10m: String,
    wind_gusts_10m: String,
}

#[derive(Debug, Deserialize)]
struct DailyWeather {
    temperature_2m_max: Vec<f64>,
    temperature_2m_min: Vec<f64>,
    precipitation_probability_max: Vec<f64>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DailyUnits {
    temperature_2m_max: String,
    temperature_2m_min: String,
    precipitation_probability_max: String,
}

#[derive(Debug, Serialize)]
struct JsonOutput<'a> {
    location: &'a Location,
    condition: &'static str,
    timezone: &'a str,
    timezone_abbreviation: &'a str,
    current: &'a CurrentWeather,
    current_units: &'a CurrentUnits,
    today: TodayOutput<'a>,
}

#[derive(Debug, Serialize)]
struct TodayOutput<'a> {
    temperature_max: f64,
    temperature_min: f64,
    temperature_unit: &'a str,
    precipitation_probability_max: f64,
    precipitation_probability_unit: &'a str,
    sunrise: &'a str,
    sunset: &'a str,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    validate_coordinates(args.latitude, args.longitude)?;

    let client = Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(concat!("lookoutside/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("could not create the HTTP client")?;

    let location = resolve_location(&client, &args)?;
    let weather = fetch_weather(&client, &location, args.units)?;

    if args.json {
        print_json(&location, &weather)?;
    } else {
        let color =
            !args.no_color && env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
        print_weather_card(&location, &weather, color);
    }

    Ok(())
}

fn validate_coordinates(latitude: Option<f64>, longitude: Option<f64>) -> Result<()> {
    if let Some(latitude) = latitude {
        if !(-90.0..=90.0).contains(&latitude) {
            bail!("latitude must be between -90 and 90 degrees");
        }
    }
    if let Some(longitude) = longitude {
        if !(-180.0..=180.0).contains(&longitude) {
            bail!("longitude must be between -180 and 180 degrees");
        }
    }
    Ok(())
}

fn resolve_location(client: &Client, args: &Args) -> Result<Location> {
    match (&args.place, args.latitude, args.longitude) {
        (Some(place), _, _) => geocode_place(client, place),
        (None, Some(latitude), Some(longitude)) => Ok(Location {
            name: format!("{latitude:.4}, {longitude:.4}"),
            latitude,
            longitude,
            source: "coordinates",
        }),
        _ => detect_ip_location(client),
    }
}

fn detect_ip_location(client: &Client) -> Result<Location> {
    let response = client
        .get(endpoint("LOOKOUTSIDE_IP_URL", IP_LOCATION_URL))
        .query(&[(
            "fields",
            "success,message,city,region,country,latitude,longitude,flag.emoji",
        )])
        .send()
        .context("could not detect your approximate location")?
        .error_for_status()
        .context("the IP location service returned an error")?
        .json::<IpLocationResponse>()
        .context("the IP location service returned an unexpected response")?;

    if !response.success {
        bail!(
            "could not detect your location: {}. Try passing a place name or --lat/--lon",
            response
                .message
                .as_deref()
                .unwrap_or("unknown location error")
        );
    }

    let latitude = response
        .latitude
        .context("location response had no latitude")?;
    let longitude = response
        .longitude
        .context("location response had no longitude")?;
    let name = join_location_parts([
        response.city.as_deref(),
        response.region.as_deref(),
        response.country.as_deref(),
    ]);
    let flag = response
        .flag
        .and_then(|flag| flag.emoji)
        .unwrap_or_default();
    let name = if flag.is_empty() {
        name
    } else {
        format!("{name} {flag}")
    };

    Ok(Location {
        name,
        latitude,
        longitude,
        source: "ip",
    })
}

fn geocode_place(client: &Client, place: &str) -> Result<Location> {
    let response = client
        .get(endpoint("LOOKOUTSIDE_GEOCODING_URL", GEOCODING_URL))
        .query(&[
            ("name", place),
            ("count", "1"),
            ("language", "en"),
            ("format", "json"),
        ])
        .send()
        .with_context(|| format!("could not search for {place:?}"))?
        .error_for_status()
        .context("the place search service returned an error")?
        .json::<GeocodingResponse>()
        .context("the place search service returned an unexpected response")?;

    let result = response
        .results
        .and_then(|results| results.into_iter().next())
        .with_context(|| format!("no place found for {place:?}"))?;

    let country = result
        .country
        .or(result.country_code)
        .filter(|country| !country.eq_ignore_ascii_case(&result.name));
    let name = join_location_parts([
        Some(result.name.as_str()),
        result.admin1.as_deref(),
        country.as_deref(),
    ]);

    Ok(Location {
        name,
        latitude: result.latitude,
        longitude: result.longitude,
        source: "search",
    })
}

fn fetch_weather(client: &Client, location: &Location, units: Units) -> Result<WeatherResponse> {
    client
        .get(endpoint("LOOKOUTSIDE_FORECAST_URL", FORECAST_URL))
        .query(&[
            ("latitude", location.latitude.to_string()),
            ("longitude", location.longitude.to_string()),
            ("current", "temperature_2m,relative_humidity_2m,apparent_temperature,is_day,precipitation,weather_code,cloud_cover,pressure_msl,wind_speed_10m,wind_direction_10m,wind_gusts_10m".to_owned()),
            ("daily", "temperature_2m_max,temperature_2m_min,precipitation_probability_max,sunrise,sunset".to_owned()),
            ("timezone", "auto".to_owned()),
            ("forecast_days", "1".to_owned()),
            ("temperature_unit", units.temperature_api_value().to_owned()),
            ("wind_speed_unit", units.wind_api_value().to_owned()),
            ("precipitation_unit", units.precipitation_api_value().to_owned()),
        ])
        .send()
        .context("could not reach the weather service")?
        .error_for_status()
        .context("the weather service returned an error")?
        .json::<WeatherResponse>()
        .context("the weather service returned an unexpected response")
}

fn endpoint(variable: &str, default: &'static str) -> String {
    env::var(variable).unwrap_or_else(|_| default.to_owned())
}

fn print_json(location: &Location, weather: &WeatherResponse) -> Result<()> {
    let today = today(weather)?;
    let output = JsonOutput {
        location,
        condition: weather_description(weather.current.weather_code),
        timezone: &weather.timezone,
        timezone_abbreviation: &weather.timezone_abbreviation,
        current: &weather.current,
        current_units: &weather.current_units,
        today,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn today(weather: &WeatherResponse) -> Result<TodayOutput<'_>> {
    Ok(TodayOutput {
        temperature_max: first(
            &weather.daily.temperature_2m_max,
            "daily maximum temperature",
        )?,
        temperature_min: first(
            &weather.daily.temperature_2m_min,
            "daily minimum temperature",
        )?,
        temperature_unit: &weather.daily_units.temperature_2m_max,
        precipitation_probability_max: first(
            &weather.daily.precipitation_probability_max,
            "daily precipitation probability",
        )?,
        precipitation_probability_unit: &weather.daily_units.precipitation_probability_max,
        sunrise: first_str(&weather.daily.sunrise, "sunrise")?,
        sunset: first_str(&weather.daily.sunset, "sunset")?,
    })
}

fn print_weather_card(location: &Location, weather: &WeatherResponse, color: bool) {
    let current = &weather.current;
    let units = &weather.current_units;
    let (icon, description) = weather_condition(current.weather_code, current.is_day == 1);
    let wind_direction = compass_direction(current.wind_direction_10m);
    let heading = paint("LOOK OUTSIDE", "1;36", color);
    let location_name = paint(&location.name, "1", color);
    let temperature = paint(
        &format!("{:.0}{}", current.temperature_2m, units.temperature_2m),
        "1;33",
        color,
    );

    println!("{heading}");
    println!("📍 {location_name}");
    println!();
    println!("  {icon}  {temperature}  {description}");
    println!(
        "     Feels like {:.0}{} · High {:.0}{} / Low {:.0}{}",
        current.apparent_temperature,
        units.apparent_temperature,
        weather
            .daily
            .temperature_2m_max
            .first()
            .unwrap_or(&f64::NAN),
        weather.daily_units.temperature_2m_max,
        weather
            .daily
            .temperature_2m_min
            .first()
            .unwrap_or(&f64::NAN),
        weather.daily_units.temperature_2m_min,
    );
    println!();
    println!(
        "  💧 Humidity {:>3.0}{}       🌧  Precipitation {:.1}{}",
        current.relative_humidity_2m,
        units.relative_humidity_2m,
        current.precipitation,
        units.precipitation,
    );
    println!(
        "  💨 Wind {:>5.1}{} {:>3}   ☁️  Cloud cover {:.0}{}",
        current.wind_speed_10m,
        units.wind_speed_10m,
        wind_direction,
        current.cloud_cover,
        units.cloud_cover,
    );
    println!(
        "     Gusts {:>4.1}{}         ◉  Pressure {:.0}{}",
        current.wind_gusts_10m, units.wind_gusts_10m, current.pressure_msl, units.pressure_msl,
    );

    if let Some(chance) = weather.daily.precipitation_probability_max.first() {
        println!(
            "  ☂️  Today's rain chance: {:.0}{}",
            chance, weather.daily_units.precipitation_probability_max
        );
    }

    let sunrise = weather.daily.sunrise.first().map(|value| short_time(value));
    let sunset = weather.daily.sunset.first().map(|value| short_time(value));
    if let (Some(sunrise), Some(sunset)) = (sunrise, sunset) {
        println!("  🌅 Sunrise {sunrise} · Sunset {sunset}");
    }

    println!();
    println!(
        "  Updated {} {} · {:.3}, {:.3}",
        short_datetime(&current.time),
        weather.timezone_abbreviation,
        location.latitude,
        location.longitude
    );
}

fn first(values: &[f64], name: &str) -> Result<f64> {
    values
        .first()
        .copied()
        .with_context(|| format!("weather response had no {name}"))
}

fn first_str<'a>(values: &'a [String], name: &str) -> Result<&'a str> {
    values
        .first()
        .map(String::as_str)
        .with_context(|| format!("weather response had no {name}"))
}

fn join_location_parts<const N: usize>(parts: [Option<&str>; N]) -> String {
    let mut unique = Vec::new();
    for part in parts
        .into_iter()
        .flatten()
        .filter(|part| !part.trim().is_empty())
    {
        if !unique
            .iter()
            .any(|saved: &&str| saved.eq_ignore_ascii_case(part))
        {
            unique.push(part);
        }
    }
    if unique.is_empty() {
        "Unknown location".to_owned()
    } else {
        unique.join(", ")
    }
}

fn weather_condition(code: u16, is_day: bool) -> (&'static str, &'static str) {
    let description = weather_description(code);
    let icon = match code {
        0 if is_day => "☀️",
        0 => "🌙",
        1 | 2 if is_day => "🌤️",
        1 | 2 => "☁️",
        3 => "☁️",
        45 | 48 => "🌫️",
        51 | 53 | 55 | 56 | 57 => "🌦️",
        61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => "🌧️",
        71 | 73 | 75 | 77 | 85 | 86 => "🌨️",
        95 | 96 | 99 => "⛈️",
        _ => "🌡️",
    };
    (icon, description)
}

fn weather_description(code: u16) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Rime fog",
        51 => "Light drizzle",
        53 => "Drizzle",
        55 => "Heavy drizzle",
        56 => "Light freezing drizzle",
        57 => "Heavy freezing drizzle",
        61 => "Light rain",
        63 => "Rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Light snow",
        73 => "Snow",
        75 => "Heavy snow",
        77 => "Snow grains",
        80 => "Light rain showers",
        81 => "Rain showers",
        82 => "Heavy rain showers",
        85 => "Light snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm with hail",
        99 => "Severe thunderstorm with hail",
        _ => "Unknown conditions",
    }
}

fn compass_direction(degrees: f64) -> &'static str {
    const DIRECTIONS: [&str; 16] = [
        "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE", "S", "SSW", "SW", "WSW", "W", "WNW",
        "NW", "NNW",
    ];
    let normalized = degrees.rem_euclid(360.0);
    DIRECTIONS[((normalized / 22.5).round() as usize) % DIRECTIONS.len()]
}

fn short_time(value: &str) -> &str {
    value
        .rsplit_once('T')
        .map(|(_, time)| time)
        .unwrap_or(value)
}

fn short_datetime(value: &str) -> String {
    value.replace('T', " ")
}

fn paint(text: &str, code: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_coordinate_ranges() {
        assert!(validate_coordinates(Some(90.0), Some(-180.0)).is_ok());
        assert!(validate_coordinates(Some(90.1), Some(0.0)).is_err());
        assert!(validate_coordinates(Some(0.0), Some(-180.1)).is_err());
    }

    #[test]
    fn maps_wmo_weather_codes() {
        assert_eq!(weather_condition(0, true), ("☀️", "Clear sky"));
        assert_eq!(weather_condition(0, false), ("🌙", "Clear sky"));
        assert_eq!(weather_description(82), "Heavy rain showers");
        assert_eq!(weather_description(999), "Unknown conditions");
    }

    #[test]
    fn maps_wind_to_compass_points() {
        assert_eq!(compass_direction(0.0), "N");
        assert_eq!(compass_direction(45.0), "NE");
        assert_eq!(compass_direction(181.0), "S");
        assert_eq!(compass_direction(359.0), "N");
        assert_eq!(compass_direction(-90.0), "W");
    }

    #[test]
    fn joins_only_distinct_location_parts() {
        assert_eq!(
            join_location_parts([Some("Paris"), Some("Île-de-France"), Some("France")]),
            "Paris, Île-de-France, France"
        );
        assert_eq!(
            join_location_parts([Some("Singapore"), Some("Singapore"), None]),
            "Singapore"
        );
    }

    #[test]
    fn formats_api_timestamps() {
        assert_eq!(short_time("2025-06-01T05:12"), "05:12");
        assert_eq!(short_datetime("2025-06-01T05:12"), "2025-06-01 05:12");
    }
}
