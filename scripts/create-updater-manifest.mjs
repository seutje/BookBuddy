import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { Buffer } from "node:buffer";

export function updaterManifest(version, signature, notes, date) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version))
    throw new Error("A stable canonical version is required.");
  const decoded = Buffer.from(signature.trim(), "base64").toString("utf8");
  const comment = decoded
    .split("\n")
    .find((line) => line.startsWith("trusted comment: "));
  if (!comment?.split("\t").includes(`version:${version}`))
    throw new Error("The installer signature must bind this release version.");
  return {
    version,
    notes,
    pub_date: date.toISOString(),
    platforms: {
      "windows-x86_64": {
        signature: signature.trim(),
        url: `https://github.com/seutje/BookBuddy/releases/download/v${version}/BookBuddy_${version}_x64-setup.exe`,
      },
    },
  };
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const version = JSON.parse(readFileSync("package.json", "utf8")).version;
  if (process.argv[2] !== `v${version}`)
    throw new Error("The release tag must match the package version.");
  const directory = "src-tauri/target/release/bundle/nsis";
  const signature = readFileSync(
    join(directory, `BookBuddy_${version}_x64-setup.exe.sig`),
    "utf8",
  );
  const manifest = updaterManifest(
    version,
    signature,
    process.env.RELEASE_NOTES ??
      `BookBuddy ${version}. Save your edits before installing. A complete local recovery backup is created before the update starts.`,
    new Date(),
  );
  writeFileSync(
    join(directory, "latest.json"),
    JSON.stringify(manifest, null, 2) + "\n",
  );
}
