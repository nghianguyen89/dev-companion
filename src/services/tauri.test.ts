import { describe, expect, it, vi } from "vitest";
import type { CompressionConfig, FileTransferConfig } from "../types/codex";

const { invoke } = vi.hoisted(() => { vi.stubGlobal("window", { __TAURI_INTERNALS__: {} }); return { invoke: vi.fn() }; });
vi.mock("@tauri-apps/api/core", () => ({ invoke, Channel: class<T = unknown> { onmessage: (payload: T) => void; constructor(onmessage: (payload: T) => void) { this.onmessage = onmessage; } } }));
import { deleteFileTransferLog, previewCompression, previewFileTransfer, restoreCodexMigration, startCompression, startFileTransfer } from "./tauri";

describe("file transfer IPC", () => {
  it("sends boolean confirmation fields", async () => {
    const config = { source: "C:\\", destination: "E:\\copy", mode: "fastCopy", includeSubfolders: {}, preserveTimestamps: {}, skipJunctionPoints: {}, restartable: {}, copyEmptyDirectories: {}, verifyDestination: {}, saveLog: {}, shutdownWhenFinished: {}, threads: 8, retries: 1, retryWait: 1, excludeFolders: [], excludeFiles: [], selectionEnabled: {}, selectedEntries: [], mirrorConfirmed: {}, systemLocationConfirmed: {}, destinationDataConfirmed: {} } as unknown as FileTransferConfig;
    await previewFileTransfer(config);
    expect(invoke).toHaveBeenCalledWith("preview_file_transfer", { config: expect.objectContaining({ includeSubfolders: false, selectionEnabled: false, mirrorConfirmed: false, systemLocationConfirmed: false, destinationDataConfirmed: false }) });
    await startFileTransfer(config, false, 123);
    expect(invoke).toHaveBeenLastCalledWith("start_file_transfer", { config: expect.objectContaining({ includeSubfolders: false, selectionEnabled: false }), analyze: false, progressTotalBytes: 123 });
    await deleteFileTransferLog("transfer-1");
    expect(invoke).toHaveBeenLastCalledWith("delete_file_transfer_log", { id: "transfer-1" });
  });
});

describe("compression IPC", () => {
  it("sends relative and regex exclusions to preview and start", async () => {
    const config: CompressionConfig = { source: "C:\\source", outputFolder: "C:\\", mode: "strong", excludedPaths: ["build/cache"], regexExclusions: ["(?i)\\.log$"] };
    await previewCompression(config);
    expect(invoke).toHaveBeenCalledWith("preview_compression", { config });
    await startCompression(config);
    expect(invoke).toHaveBeenLastCalledWith("start_compression", { config });
  });
});

describe("Codex migration restore IPC", () => {
  it("forwards progress and ignores channel events after native restore resolves", async () => {
    let resolveInvoke!: (value: unknown) => void;
    invoke.mockReturnValueOnce(new Promise((resolve) => { resolveInvoke = resolve; }));
    const onProgress = vi.fn();
    const pending = restoreCodexMigration("token", "REPLACE CODEX", true, onProgress);
    const args = invoke.mock.lastCall?.[1] as { onProgress: { onmessage: (value: unknown) => void } };
    const progress = { stage: "restoring", completed: 2, total: 5, file: "accounts.json", operation: "copy-read", bytesCopied: 42, fileBytes: 100 };
    args.onProgress.onmessage(progress);
    expect(onProgress).toHaveBeenCalledWith(progress);
    expect(invoke).toHaveBeenLastCalledWith("restore_codex_migration", { token: "token", confirmation: "REPLACE CODEX", replaceChat: true, onProgress: args.onProgress });
    resolveInvoke({ restored: 2, skipped: 3, errors: [], rollbackRemaining: 0, safetyArchive: null });
    await pending;
    args.onProgress.onmessage({ ...progress, bytesCopied: 99 });
    expect(onProgress).toHaveBeenCalledTimes(1);
  });

  it("stops forwarding progress after native restore rejects", async () => {
    let rejectInvoke!: (reason: unknown) => void;
    invoke.mockReturnValueOnce(new Promise((_resolve, reject) => { rejectInvoke = reject; }));
    const onProgress = vi.fn();
    const pending = restoreCodexMigration("token", "REPLACE CODEX", true, onProgress);
    const args = invoke.mock.lastCall?.[1] as { onProgress: { onmessage: (value: unknown) => void } };
    const progress = { stage: "restoring", completed: 1, total: 5 };
    args.onProgress.onmessage(progress);
    expect(onProgress).toHaveBeenCalledWith(progress);
    rejectInvoke(new Error("native failure"));
    await expect(pending).rejects.toThrow("native failure");
    args.onProgress.onmessage({ ...progress, completed: 2 });
    expect(onProgress).toHaveBeenCalledTimes(1);
  });
});

describe("browser preview", () => {
  it("provides safe display-only defaults without Tauri internals", async () => {
    vi.stubGlobal("window", {});
    vi.resetModules();
    const { getConfiguration, getDiagnostics } = await import("./tauri");
    await expect(getConfiguration()).resolves.toMatchObject({ language: "en", theme: "system" });
    await expect(getDiagnostics()).resolves.toMatchObject({ operatingSystem: "unknown", codexHomeExists: false });
    vi.unstubAllGlobals();
  });
});
