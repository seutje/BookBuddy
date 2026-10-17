import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { updaterManifest } from "./create-updater-manifest.mjs";

const signature = readFileSync(
  "tests/fixtures/updater-payload.txt.sig",
  "utf8",
);
test("manifest pins BookBuddy's platform, origin and signed version", () => {
  const result = updaterManifest(
    "0.2.0",
    signature,
    "<b>Literal notes</b>",
    new Date("2026-10-17T00:00:00Z"),
  );
  assert.equal(result.version, "0.2.0");
  assert.equal(result.notes, "<b>Literal notes</b>");
  assert.equal(result.platforms["windows-x86_64"].signature, signature.trim());
  assert.equal(
    result.platforms["windows-x86_64"].url,
    "https://github.com/seutje/BookBuddy/releases/download/v0.2.0/BookBuddy_0.2.0_x64-setup.exe",
  );
});
test("unsigned, relabelled or noncanonical manifests are rejected", () => {
  for (const version of ["0.3.0", "00.2.0", "0.2.0-beta", "../0.2.0"])
    assert.throws(() => updaterManifest(version, signature, "", new Date()));
  assert.throws(() => updaterManifest("0.2.0", "", "", new Date()));
});
test("CI publishes the signature and updater feed with a dedicated signing secret", async () => {
  const yaml = await import("js-yaml");
  const workflow = yaml.load(
    readFileSync(".github/workflows/release.yml", "utf8"),
  );
  const build = workflow.jobs.build;
  const signing = build.steps.find(
    (step) => step.name === "Build NSIS installer",
  );
  assert.equal(
    signing.env.TAURI_SIGNING_PRIVATE_KEY,
    "${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}",
  );
  assert.ok(signing.run.includes("create-updater-manifest.mjs"));
  const upload = build.steps.find((step) =>
    step.uses?.startsWith("actions/upload-artifact@"),
  );
  assert.ok(upload.with.path.includes(".exe.sig"));
  assert.ok(upload.with.path.includes("latest.json"));
});
