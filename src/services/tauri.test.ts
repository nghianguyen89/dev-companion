import { describe, expect, it, vi } from "vitest";
import type { FileTransferConfig } from "../types/codex";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
import { previewFileTransfer, startFileTransfer } from "./tauri";

describe("file transfer IPC", () => {
  it("sends boolean confirmation fields", async () => {
    const config = { source: "C:\\", destination: "E:\\copy", mode: "fastCopy", includeSubfolders: {}, preserveTimestamps: {}, skipJunctionPoints: {}, restartable: {}, copyEmptyDirectories: {}, verifyDestination: {}, saveLog: {}, shutdownWhenFinished: {}, threads: 8, retries: 1, retryWait: 1, excludeFolders: [], excludeFiles: [], selectionEnabled: {}, selectedEntries: [], mirrorConfirmed: {}, systemLocationConfirmed: {}, destinationDataConfirmed: {} } as unknown as FileTransferConfig;
    await previewFileTransfer(config);
    expect(invoke).toHaveBeenCalledWith("preview_file_transfer", { config: expect.objectContaining({ includeSubfolders: false, selectionEnabled: false, mirrorConfirmed: false, systemLocationConfirmed: false, destinationDataConfirmed: false }) });
    await startFileTransfer(config, false, 123);
    expect(invoke).toHaveBeenLastCalledWith("start_file_transfer", { config: expect.objectContaining({ includeSubfolders: false, selectionEnabled: false }), analyze: false, progressTotalBytes: 123 });
  });
});
