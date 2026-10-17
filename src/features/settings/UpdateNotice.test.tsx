import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { UpdateNotice } from "./UpdateNotice";
import { checkUpdate, installUpdate, startupUpdate } from "../../lib/updates";
vi.mock("../../lib/updates", () => ({
  checkUpdate: vi.fn(),
  installUpdate: vi.fn(),
  startupUpdate: vi.fn(),
}));
beforeEach(() => {
  vi.clearAllMocks();
  HTMLDialogElement.prototype.showModal = function () {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function () {
    this.open = false;
  };
});
afterEach(cleanup);
const update = { version: "0.2.0", notes: "<img src=x onerror=alert(1)>" };
test("startup offline failure is silent", async () => {
  vi.mocked(startupUpdate).mockRejectedValue(new Error("offline"));
  render(<UpdateNotice />);
  await waitFor(() => expect(startupUpdate).toHaveBeenCalled());
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.queryByRole("dialog")).toBeNull();
});
test("startup notes remain literal and postponement never downloads", async () => {
  vi.mocked(startupUpdate).mockResolvedValue({ enabled: true, update });
  render(<UpdateNotice />);
  const notes = await screen.findByText(update.notes);
  expect(notes.querySelector("img")).toBeNull();
  fireEvent.click(screen.getByText("Not now"));
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(installUpdate).not.toHaveBeenCalled();
});
test("manual retry reports offline failure then up-to-date status", async () => {
  vi.mocked(checkUpdate)
    .mockRejectedValueOnce(new Error("offline"))
    .mockResolvedValueOnce({ enabled: true, update: null });
  render(<UpdateNotice manual />);
  fireEvent.click(screen.getByText("Check for updates"));
  expect(await screen.findByRole("alert")).toBeTruthy();
  fireEvent.click(screen.getByText("Check for updates"));
  expect(await screen.findByText("BookBuddy is up to date.")).toBeTruthy();
});
test("Escape postpones an offered update without downloading", async () => {
  vi.mocked(startupUpdate).mockResolvedValue({ enabled: true, update });
  render(<UpdateNotice />);
  const dialog = await screen.findByRole("dialog");
  fireEvent(dialog, new Event("cancel", { cancelable: true }));
  expect(screen.queryByRole("dialog")).toBeNull();
  expect(installUpdate).not.toHaveBeenCalled();
});
test("explicit install shows progress, blocks dismissal and recovers after failure", async () => {
  vi.mocked(checkUpdate).mockResolvedValue({ enabled: true, update });
  let reject!: (error: Error) => void;
  vi.mocked(installUpdate).mockImplementation((_version, onProgress) => {
    onProgress({
      phase: "backing_up",
      downloaded: 100,
      total: 100,
      recoveryPath: null,
    });
    return new Promise((_resolve, fail) => {
      reject = fail;
    });
  });
  render(<UpdateNotice manual />);
  fireEvent.click(screen.getByText("Check for updates"));
  fireEvent.click(await screen.findByText("Download and install"));
  expect((screen.getByText("Not now") as HTMLButtonElement).disabled).toBe(
    true,
  );
  expect(screen.getByText("Saving recovery backup…")).toBeTruthy();
  fireEvent(
    screen.getByRole("dialog"),
    new Event("cancel", { cancelable: true }),
  );
  expect(screen.getByRole("dialog")).toBeTruthy();
  reject(new Error("verification failed"));
  expect(await screen.findByRole("alert")).toBeTruthy();
  expect((screen.getByText("Not now") as HTMLButtonElement).disabled).toBe(
    false,
  );
});
