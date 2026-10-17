import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import process from "node:process";
import { test } from "node:test";
import { fileURLToPath, URL } from "node:url";

const script = fileURLToPath(
  new URL("./check-release-version.mjs", import.meta.url),
);
function check(tag, mismatch = null) {
  const root = mkdtempSync(join(tmpdir(), "bookbuddy-release-"));
  const version = (file) => (file === mismatch ? "0.1.0" : "0.1.1");
  try {
    mkdirSync(join(root, "src-tauri"));
    for (const file of ["package.json", "src-tauri/tauri.conf.json"]) {
      writeFileSync(
        join(root, file),
        JSON.stringify({ version: version(file) }),
      );
    }
    writeFileSync(
      join(root, "package-lock.json"),
      JSON.stringify({
        version: version("lock"),
        packages: { "": { version: version("lock-root") } },
      }),
    );
    writeFileSync(
      join(root, "src-tauri/Cargo.toml"),
      `[package]\nname = "bookbuddy"\nversion = "${version("cargo")}"\n\n[dependencies]\nserde = "1"\n`,
    );
    writeFileSync(
      join(root, "src-tauri/Cargo.lock"),
      `version = 4\n\n[[package]]\nname = "bookbuddy"\nversion = "${version("cargo-lock")}"\n`,
    );
    return spawnSync(process.execPath, [script, tag], {
      cwd: root,
      encoding: "utf8",
    });
  } finally {
    assert.ok(
      resolve(root).startsWith(resolve(tmpdir(), "bookbuddy-release-")),
    );
    rmSync(root, { recursive: true, force: true });
  }
}
test("accepts matching stable release versions", () => {
  const result = check("v0.1.1");
  assert.equal(result.status, 0, result.stderr);
});
test("rejects malformed, prerelease and noncanonical tags", () => {
  for (const tag of [
    "0.1.1",
    "v0.1",
    "v01.1.1",
    "v0.1.1-beta.1",
    "v0.1.1/extra",
  ]) {
    assert.notEqual(check(tag).status, 0, tag);
  }
});
test("rejects every mismatched manifest and lockfile", () => {
  for (const file of [
    "package.json",
    "lock",
    "lock-root",
    "src-tauri/tauri.conf.json",
    "cargo",
    "cargo-lock",
  ]) {
    const result = check("v0.1.1", file);
    assert.notEqual(result.status, 0, file);
    assert.match(result.stderr, /requires 0\.1\.1/);
  }
});
