import { execFileSync, spawnSync } from "node:child_process";
import { appendFileSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

export async function resolveReleaseTarget(context, dependencies) {
  const recovery = context.eventName === "push" && context.refType === "branch";
  const tag =
    context.eventName === "workflow_dispatch"
      ? context.inputTag
      : recovery
        ? `v${context.version}`
        : context.refName;
  if (!/^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(tag ?? "")) {
    if (context.eventName === "workflow_dispatch")
      throw new Error("Choose a stable vMAJOR.MINOR.PATCH tag.");
    return { publish: false, reason: "No stable release tag selected." };
  }
  const sha = await dependencies.findTag(tag);
  if (!sha) {
    if (!recovery) throw new Error(`Release tag ${tag} does not exist.`);
    return {
      publish: false,
      tag,
      reason: "The current version has not been tagged.",
    };
  }
  if (!/^[a-f0-9]{40}$/.test(sha))
    throw new Error("The release tag did not resolve to a commit.");
  if (recovery && !(await dependencies.isAncestor(sha))) {
    return {
      publish: false,
      tag,
      reason: "The version tag is outside the current branch history.",
    };
  }
  if (await dependencies.releasePublished(tag)) {
    return {
      publish: false,
      tag,
      reason: "This release is already published.",
    };
  }
  return {
    publish: true,
    tag,
    sha,
    reason: "Build the existing tagged commit.",
  };
}

async function main() {
  const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, "utf8"));
  const result = await resolveReleaseTarget(
    {
      eventName: process.env.GITHUB_EVENT_NAME,
      refType: process.env.GITHUB_REF_TYPE,
      refName: process.env.GITHUB_REF_NAME,
      inputTag: event.inputs?.tag,
      version: JSON.parse(readFileSync("package.json", "utf8")).version,
    },
    {
      findTag: (tag) => {
        try {
          return execFileSync(
            "git",
            ["rev-parse", "--verify", `refs/tags/${tag}^{commit}`],
            { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
          ).trim();
        } catch (error) {
          if (error.status === 128) return null;
          throw error;
        }
      },
      isAncestor: (sha) => {
        const result = spawnSync("git", [
          "merge-base",
          "--is-ancestor",
          sha,
          "HEAD",
        ]);
        if (result.status !== 0 && result.status !== 1)
          throw new Error("Could not verify the tagged commit's ancestry.");
        return result.status === 0;
      },
      releasePublished: async (tag) => {
        const response = await globalThis.fetch(
          `${process.env.GITHUB_API_URL}/repos/${process.env.GITHUB_REPOSITORY}/releases/tags/${encodeURIComponent(tag)}`,
          {
            headers: {
              Accept: "application/vnd.github+json",
              Authorization: `Bearer ${process.env.GH_TOKEN}`,
            },
          },
        );
        if (response.status === 404) return false;
        if (!response.ok)
          throw new Error(
            `Release lookup failed with HTTP ${response.status}; refusing to assume a release is absent.`,
          );
        return !(await response.json()).draft;
      },
    },
  );
  appendFileSync(
    process.env.GITHUB_OUTPUT,
    `publish=${result.publish}\ntag=${result.tag ?? ""}\nsha=${result.sha ?? ""}\n`,
  );
  process.stdout.write(`${result.tag ?? "Release"}: ${result.reason}\n`);
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  main().catch((error) => {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  });
}
