import assert from "node:assert/strict";
import { test } from "node:test";
import { resolveReleaseTarget } from "./resolve-release.mjs";

const sha = "a".repeat(40);
const branch = {
  eventName: "push",
  refType: "branch",
  refName: "main",
  version: "0.1.1",
};
const dependencies = {
  findTag: async () => sha,
  isAncestor: async () => true,
  releasePublished: async () => false,
};
test("a main push recovers an existing unpublished version tag without moving it", async () => {
  const result = await resolveReleaseTarget(branch, dependencies);
  assert.equal(result.publish, true);
  assert.equal(result.tag, "v0.1.1");
  assert.equal(result.sha, sha);
});
test("a tag push builds that tag rather than the default branch version", async () => {
  const result = await resolveReleaseTarget(
    { ...branch, refType: "tag", refName: "v0.2.0" },
    dependencies,
  );
  assert.equal(result.publish, true);
  assert.equal(result.tag, "v0.2.0");
});
test("manual recovery selects the requested existing tag", async () => {
  const result = await resolveReleaseTarget(
    { ...branch, eventName: "workflow_dispatch", inputTag: "v0.1.0" },
    dependencies,
  );
  assert.equal(result.tag, "v0.1.0");
  assert.equal(result.publish, true);
});
test("main pushes do not publish untagged versions, unrelated histories or existing releases", async () => {
  for (const change of [
    { findTag: async () => null },
    { isAncestor: async () => false },
    { releasePublished: async () => true },
  ]) {
    assert.equal(
      (await resolveReleaseTarget(branch, { ...dependencies, ...change }))
        .publish,
      false,
    );
  }
});
test("invalid tag pushes skip safely; invalid or missing explicit targets fail", async () => {
  assert.equal(
    (
      await resolveReleaseTarget(
        { ...branch, refType: "tag", refName: "v0.1.1-beta" },
        dependencies,
      )
    ).publish,
    false,
  );
  await assert.rejects(
    resolveReleaseTarget(
      { ...branch, eventName: "workflow_dispatch", inputTag: "../main" },
      dependencies,
    ),
  );
  await assert.rejects(
    resolveReleaseTarget(
      { ...branch, refType: "tag", refName: "v0.1.1" },
      { ...dependencies, findTag: async () => null },
    ),
  );
});
test("lookup failures propagate instead of risking duplicate publication", async () => {
  await assert.rejects(
    resolveReleaseTarget(branch, {
      ...dependencies,
      releasePublished: async () => {
        throw new Error("HTTP 403");
      },
    }),
    /HTTP 403/,
  );
});
