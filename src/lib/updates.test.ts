import { beforeEach, expect, test, vi } from "vitest";
const { invoke, isTauri } = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}));
vi.mock("@tauri-apps/api/core", () => ({ invoke, isTauri, Channel: class {} }));

beforeEach(() => {
  vi.resetModules();
  vi.clearAllMocks();
});
test("browser preview makes no native update request", async () => {
  isTauri.mockReturnValue(false);
  const { checkUpdate } = await import("./updates");
  expect(await checkUpdate()).toEqual({ enabled: false, update: null });
  expect(invoke).not.toHaveBeenCalled();
});
test("StrictMode subscribers share one startup request and receive its result", async () => {
  isTauri.mockReturnValue(true);
  const result = {
    enabled: true,
    update: { version: "0.2.0", notes: "Fixture" },
  };
  invoke.mockResolvedValue(result);
  const { startupUpdate } = await import("./updates");
  const first = startupUpdate();
  expect(startupUpdate()).toBe(first);
  expect(await first).toEqual(result);
  expect(invoke).toHaveBeenCalledExactlyOnceWith("check_app_update");
});
test("installation sends only the reviewed version, consent and progress channel", async () => {
  const { installUpdate } = await import("./updates");
  await installUpdate("0.2.0", vi.fn());
  expect(invoke).toHaveBeenCalledWith("install_app_update", {
    version: "0.2.0",
    confirmed: true,
    progress: expect.any(Object),
  });
});
