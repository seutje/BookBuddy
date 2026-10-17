import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
export type Release = { version: string; notes: string };
export type UpdateCheck = { enabled: boolean; update: Release | null };
export type UpdateProgress = {
  phase: "downloading" | "backing_up" | "installing";
  downloaded: number;
  total: number | null;
  recoveryPath: string | null;
};
export function checkUpdate(): Promise<UpdateCheck> {
  return isTauri()
    ? invoke("check_app_update")
    : Promise.resolve({ enabled: false, update: null });
}
// The cached promise supplies one startup request across React StrictMode's
// setup/cleanup replay. Each mounted subscriber independently ignores cleanup.
let startup: Promise<UpdateCheck> | undefined;
export function startupUpdate() {
  return (startup ??= checkUpdate());
}
export function installUpdate(
  version: string,
  onProgress: (value: UpdateProgress) => void,
): Promise<void> {
  const progress = new Channel<UpdateProgress>();
  progress.onmessage = onProgress;
  return invoke("install_app_update", { version, confirmed: true, progress });
}
