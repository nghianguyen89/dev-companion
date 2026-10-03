import { describe, expect, it, vi } from "vitest";
import type { CompressionConfig, FileTransferConfig } from "../types/codex";

const { invoke } = vi.hoisted(() => { vi.stubGlobal("window", { __TAURI_INTERNALS__: {} }); return { invoke: vi.fn() }; });
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
import { deleteFileTransferLog, previewCompression, previewFileTransfer, startCompression, startFileTransfer } from "./tauri";

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
