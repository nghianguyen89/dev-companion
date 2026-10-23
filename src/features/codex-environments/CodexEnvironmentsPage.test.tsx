import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, expect, it, vi } from "vitest";
import { I18nProvider, translate } from "../../i18n";
import type { CodexEnvironmentOverview } from "../../types/codex";
import { CodexEnvironmentsPage } from "./CodexEnvironmentsPage";

const state = vi.hoisted(() => ({ value: null as CodexEnvironmentOverview | null }));
vi.mock("../../hooks/useAsyncValue", () => ({ useAsyncValue: () => ({ ...state, error: null, loading: false, refresh: vi.fn() }) }));

beforeEach(() => {
  state.value = {
    cli: { installed: true, path: "C:\\nodejs\\codex.cmd", version: "codex-cli fixture", error: null },
    launcherDir: "C:\\Users\\A\\bin", launcherDirExists: true, launcherDirInUserPath: true,
    environments: [{ id: "custom", displayName: "Custom", commandAlias: "codex-cus", codexHome: "C:\\Users\\A\\.codex-cus", description: null, managesInstructions: false, launcherPath: null, createdAt: "", updatedAt: "", isDefault: false, codexHomeExists: true, agentsExists: false, loginStatus: "unknown" }],
  };
});

const render = () => renderToStaticMarkup(<I18nProvider value={{ language: "vi", t: (key, values) => translate("vi", key, values) }}><CodexEnvironmentsPage /></I18nProvider>);

it("labels unknown as login status, explains it and offers an alias check", () => {
  const html = render();
  expect(html).toContain("Đăng nhập: Chưa kiểm tra được");
  expect(html).toContain("Chưa kiểm tra được đăng nhập qua Codex CLI");
  expect(html).toContain("Kiểm tra alias");
  expect(html).toContain("tự động thêm thư mục launcher vào User PATH");
  expect(html).not.toContain("Không rõ");
  expect(html).not.toContain('disabled="">Mở Codex');
});

it("shows a failed probe separately from missing installation and disables launch", () => {
  state.value!.cli.version = null;
  state.value!.cli.error = "Codex CLI was found but --version failed.";
  const html = render();
  expect(html).toContain("Đã cài");
  expect(html).not.toContain("Chưa cài");
  expect(html).toContain("Codex CLI was found but --version failed.");
  expect(html).toContain('disabled="">Mở Codex');
});
