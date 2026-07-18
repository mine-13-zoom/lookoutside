import { chmodSync, copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const extension = process.platform === "win32" ? ".exe" : "";
const source = join(packageRoot, "target", "release", `lookoutside${extension}`);
const destination = join(
  packageRoot,
  "npm",
  "bin",
  `lookoutside-${process.platform}-${process.arch}${extension}`,
);

if (!existsSync(source)) {
  console.error(`Missing release binary at ${source}. Run cargo build --release first.`);
  process.exit(1);
}

mkdirSync(dirname(destination), { recursive: true });
copyFileSync(source, destination);
if (process.platform !== "win32") chmodSync(destination, 0o755);
console.log(`Staged ${destination}`);
