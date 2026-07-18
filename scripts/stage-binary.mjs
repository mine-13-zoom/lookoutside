import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// Maps Rust target triples to the Node.js platform/arch names used by
// npm/bin/lookoutside.js when it picks a bundled binary at runtime.
const TARGET_MAP = {
  "x86_64-unknown-linux-gnu": ["linux", "x64"],
  "aarch64-unknown-linux-gnu": ["linux", "arm64"],
  "x86_64-unknown-linux-musl": ["linux", "x64"],
  "aarch64-unknown-linux-musl": ["linux", "arm64"],
  "x86_64-apple-darwin": ["darwin", "x64"],
  "aarch64-apple-darwin": ["darwin", "arm64"],
  "x86_64-pc-windows-msvc": ["win32", "x64"],
  "aarch64-pc-windows-msvc": ["win32", "arm64"],
};

// Optional: --target <rust-triple> (or TARGET env var). Without it, stage the
// host build from target/release using the host platform/arch.
const argIndex = process.argv.indexOf("--target");
const target =
  (argIndex !== -1 && process.argv[argIndex + 1]) || process.env.TARGET || null;

let source;
let platform;
let arch;

if (target) {
  const mapped = TARGET_MAP[target];
  if (!mapped) {
    console.error(`Unknown Rust target "${target}". Add it to TARGET_MAP.`);
    process.exit(1);
  }
  [platform, arch] = mapped;
  const extension = platform === "win32" ? ".exe" : "";
  source = join(packageRoot, "target", target, "release", `lookoutside${extension}`);
} else {
  platform = process.platform;
  arch = process.arch;
  const extension = platform === "win32" ? ".exe" : "";
  source = join(packageRoot, "target", "release", `lookoutside${extension}`);
}

const extension = platform === "win32" ? ".exe" : "";
const destination = join(
  packageRoot,
  "npm",
  "bin",
  `lookoutside-${platform}-${arch}${extension}`,
);

if (!existsSync(source)) {
  console.error(`Missing release binary at ${source}. Run cargo build --release first.`);
  process.exit(1);
}

mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
if (platform !== "win32") chmodSync(destination, 0o755);
console.log(`Staged ${destination}`);
