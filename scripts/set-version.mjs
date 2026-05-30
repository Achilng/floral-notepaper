import fs from "node:fs";

const args = process.argv.slice(2);
const disableMacosSigning = args.includes("--disable-macos-signing");
const versionArg = args.find((arg) => !arg.startsWith("--"));

if (!versionArg) {
  console.error("Usage: node scripts/set-version.mjs <version> [--disable-macos-signing]");
  process.exit(1);
}

const version = versionArg.trim().replace(/^[vV]/, "");
if (!version) {
  console.error("Version cannot be empty.");
  process.exit(1);
}

const packageJsonPath = new URL("../package.json", import.meta.url);
const tauriConfigPath = new URL("../src-tauri/tauri.conf.json", import.meta.url);
const cargoTomlPath = new URL("../src-tauri/Cargo.toml", import.meta.url);

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
packageJson.version = version;
fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`);

const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"));
tauriConfig.version = version;
if (disableMacosSigning) {
  tauriConfig.bundle = tauriConfig.bundle ?? {};
  tauriConfig.bundle.macOS = tauriConfig.bundle.macOS ?? {};
  tauriConfig.bundle.macOS.signingIdentity = null;
}
fs.writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`);

const cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
const nextCargoToml = cargoToml.replace(/^version = "[^"]*"/m, `version = "${version}"`);
if (nextCargoToml === cargoToml) {
  console.error("Failed to update version in src-tauri/Cargo.toml.");
  process.exit(1);
}
fs.writeFileSync(cargoTomlPath, nextCargoToml);

console.log(
  `Synced package.json, src-tauri/tauri.conf.json, and src-tauri/Cargo.toml to ${version}.`,
);
