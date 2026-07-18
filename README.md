# lookoutside

Look outside from your terminal. This small Rust CLI shows the current weather around you. With no arguments it estimates your location from your public IP address; you can also pass a place name or exact coordinates.

```text
LOOK OUTSIDE
📍 London, England, United Kingdom

  🌤️  19°C  Mainly clear
     Feels like 18°C · High 23°C / Low 16°C

  💧 Humidity  55%       🌧  Precipitation 0.0mm
  💨 Wind  13.7km/h  NE   ☁️  Cloud cover 44%
```

## Run with npx

Once the package is published to npm:

```bash
# Automatically detect your approximate location
npx lookoutside

# Search for a place
npx lookoutside London
npx lookoutside "New York" --units imperial

# Use exact coordinates
npx lookoutside --lat 59.33 --lon 18.07

# Machine-readable output
npx lookoutside Tokyo --json
```

The package embeds a native Rust binary for the platform on which `npm pack`/`npm publish` runs. If that binary does not match the user's platform, the launcher falls back to compiling the included Rust source with Cargo.

## Install globally

```bash
npm install --global lookoutside
lookoutside
```

## Options

```text
Usage: lookoutside [OPTIONS] [PLACE]

Arguments:
  [PLACE]                City, postal code, or place name

Options:
      --lat <DEGREES>    Latitude; requires --lon
      --lon <DEGREES>    Longitude; requires --lat
  -u, --units <UNITS>    metric or imperial [default: metric]
      --json             Print machine-readable JSON
      --no-color         Disable ANSI colors
  -h, --help             Show help
  -V, --version          Show version
```

`NO_COLOR=1` is also respected.

## How location and weather work

- **Automatic location:** [ipwho.is](https://ipwho.is/) maps the caller's public IP to an approximate city and coordinates. No API key is needed. IP geolocation can be inaccurate, especially with VPNs, mobile networks, or corporate gateways.
- **Place search:** the [Open-Meteo Geocoding API](https://open-meteo.com/en/docs/geocoding-api) resolves a supplied name.
- **Weather:** the [Open-Meteo Forecast API](https://open-meteo.com/en/docs) supplies current conditions and today's summary.

Automatic detection sends the caller's public IP to ipwho.is as part of the normal network request. Pass a place or coordinates if you prefer not to use IP geolocation. Review each provider's limits and terms before high-volume or commercial use.

## Develop locally

Requirements: Rust 1.70+ and Node.js 18+.

```bash
cargo run -- London
cargo test
cargo clippy --all-targets -- -D warnings

# Exercise the npm launcher
npm run build
npm run stage
node npm/bin/lookoutside.js --help
```

## Package for npm

Keep the versions in `Cargo.toml` and `package.json` synchronized, then run:

```bash
npm pack
npm publish
```

The `prepack` script creates an optimized Rust build and stages it as `npm/bin/lookoutside-<platform>-<arch>` before npm creates the tarball.

## License

MIT
