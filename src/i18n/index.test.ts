import { describe, expect, it } from "vitest";
import { translate } from "./index";
import type { AppConfiguration } from "../types/codex";

describe("i18n", () => {
  it("defaults to English", () => expect(translate(undefined, "settings.language")).toBe("Language"));
  it("switches to Vietnamese", () => expect(translate("vi", "nav.settings")).toBe("Cài đặt"));
  it("localizes active transfer and compression controls", () => {
    expect(translate("vi", "fileTransfer.robocopyReady")).toBe("Robocopy sẵn sàng");
    expect(translate("vi", "compression.createArchive")).toBe("Tạo kho lưu trữ");
  });
  it("uses the language persisted in AppConfiguration", () => {
    const configuration: AppConfiguration = { theme: "system", portableMode: false, createSafetyBackups: true, language: "vi", logLevel: "warn", codexEnvironments: [], fileTransferProfiles: [] };
    expect(translate(configuration.language, "settings.language")).toBe("Ngôn ngữ");
  });
  it("falls back to English when the language is unknown", () => expect(translate("xx" as "en", "backup.restore")).toBe("Restore selected sessions"));
  it("returns the key when neither dictionary defines it", () => expect(translate("vi", "missing.key" as "nav.settings")).toBe("missing.key"));
  it("interpolates typed UI values", () => expect(translate("vi", "backup.selected", { count: 2 })).toBe("Đã chọn 2"));
  it("translates restore-history outcomes in both supported languages", () => {
    expect(translate("en", "backup.historyRolledBack")).toBe("Rolled back");
    expect(translate("vi", "backup.historyFailed")).toBe("Thất bại");
  });
});
