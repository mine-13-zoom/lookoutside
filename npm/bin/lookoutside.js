#!/usr/bin/env node

const { existsSync } = require("node:fs");
const { spawnSync } = require("node:child_process");
const { join, resolve } = require("node:path");

const packageRoot = resolve(__dirname, "../..");
const executable = process.platform === "win32" ? "lookoutside.exe" : "lookoutside";
const bundledName = `lookoutside-${process.platform}-${process.arch}${
  process.platform === "win32" ? ".exe" : ""
}`;

const candidates = [
  join(__dirname, bundledName),
  join(packageRoot, "target", "release", executable),
  join(packageRoot, "target", "debug", executable),
];

let binary = candidates.find(existsSync);

if (!binary) {
  const cargo = spawnSync("cargo", ["build", "--release", "--quiet"], {
    cwd: packageRoot,
    stdio: "inherit",
  });

  if (cargo.error?.code === "ENOENT") {
    console.error(
      `lookoutside: no prebuilt binary is available for ${process.platform}/${process.arch}.\n` +
        "Install Rust from https://rustup.rs and try again."
    );
    process.exit(1);
  }
  if (cargo.status !== 0) {
    process.exit(cargo.status ?? 1);
  }

  binary = join(packageRoot, "target", "release", executable);
}

const result = spawnSync(binary, process.argv.slice(2), {
  stdio: "inherit",
  argv0: "lookoutside",
});

if (result.error) {
  console.error(`lookoutside: could not start the Rust binary: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);
