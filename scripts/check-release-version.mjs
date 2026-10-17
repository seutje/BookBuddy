import { readFileSync } from "node:fs";
import process from "node:process";

try {
  const tag = process.argv[2];
  if (!/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(tag ?? "")) {
    throw new Error(
      "Release tags must use vMAJOR.MINOR.PATCH, for example v0.1.1.",
    );
  }
  const expected = tag.slice(1);
  const json = (file) => JSON.parse(readFileSync(file, "utf8"));
  const cargo = readFileSync("src-tauri/Cargo.toml", "utf8")
    .match(/^\[package\]([\s\S]*?)(?:^\[|$(?![\s\S]))/m)?.[1]
    .match(/^version\s*=\s*"([^"]+)"\s*$/m)?.[1];
  const cargoLock = readFileSync("src-tauri/Cargo.lock", "utf8").match(
    /^\[\[package\]\]\r?\nname = "bookbuddy"\r?\nversion = "([^"]+)"/m,
  )?.[1];
  const lock = json("package-lock.json");
  const versions = {
    "package.json": json("package.json").version,
    "package-lock.json": lock.version,
    "package-lock.json root package": lock.packages?.[""]?.version,
    "src-tauri/tauri.conf.json": json("src-tauri/tauri.conf.json").version,
    "src-tauri/Cargo.toml": cargo,
    "src-tauri/Cargo.lock": cargoLock,
  };
  for (const [file, version] of Object.entries(versions)) {
    if (version !== expected) {
      throw new Error(
        `${file} has version ${version ?? "missing"}; ${tag} requires ${expected}. Update and commit all versions before tagging.`,
      );
    }
  }
  process.stdout.write(
    `${tag} matches all application manifests and lockfiles.\n`,
  );
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
