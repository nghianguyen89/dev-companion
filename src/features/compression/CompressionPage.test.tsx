import { renderToStaticMarkup } from "react-dom/server";
import { CompressionPage } from "./CompressionPage";
import { I18nProvider, translate } from "../../i18n";
import type { AppConfiguration } from "../../types/codex";
import { expect, it } from "vitest";
import { compressionPathExcluded, compressionPresets, defaultCompressionExcludedPaths, toggleCompressionPath } from "./CompressionDefaults";

it("prefills the archive handoff without excluding retained build output", () => {
  const configuration: AppConfiguration = { theme: "light", portableMode: false, createSafetyBackups: true, language: "en", logLevel: "warn", codexEnvironments: [], fileTransferProfiles: [] };
  const html = renderToStaticMarkup(<CompressionPage configuration={configuration} onSaveConfiguration={async () => {}} initialSource="D:/Các dự án/archive me" />);
  expect(html).toContain("D:/Các dự án/archive me");
  const textareas = [...html.matchAll(/<textarea[^>]*>([\s\S]*?)<\/textarea>/g)];
  expect(textareas.length).toBeGreaterThan(0);
  expect(textareas.every(match => match[1] === "")).toBe(true);
});

it("excludes common generated content by default without hiding source files", () => {
  expect(defaultCompressionExcludedPaths([
    { path: "node_modules", isDirectory: true, bytes: 0 },
    { path: "node_modules/react/index.js", isDirectory: false, bytes: 1 },
    { path: "src/app.ts", isDirectory: false, bytes: 1 },
    { path: "logs", isDirectory: true, bytes: 0 },
    { path: "logs/run.log", isDirectory: false, bytes: 1 },
    { path: "debug.log", isDirectory: false, bytes: 1 },
  ])).toEqual(["node_modules", "logs", "debug.log"]);
});


it("applies profile rules at every depth and allows checkbox overrides", () => {
  const config = { source: "", outputFolder: "", mode: "fast" as const, excludedPaths: [], regexExclusions: [], excludedDirectoryNames: [...compressionPresets[0].excludedDirectoryNames], excludedFileExtensions: [...compressionPresets[0].excludedFileExtensions] };
  const folder = { path: "app/node_modules", isDirectory: true, bytes: 0 };
  const file = { path: "app/node_modules/pkg/index.js", isDirectory: false, bytes: 1 };
  expect(compressionPathExcluded(config, file)).toBe(true);
  const included = toggleCompressionPath(config, folder);
  expect(compressionPathExcluded(included, folder)).toBe(false);
  expect(compressionPathExcluded(included, file)).toBe(false);
  expect(compressionPathExcluded(toggleCompressionPath(included, folder), file)).toBe(true);
  expect(compressionPathExcluded(config, { path: "run.LOG", isDirectory: false, bytes: 1 })).toBe(true);
  expect(compressionPathExcluded(config, { path: ".env", isDirectory: false, bytes: 1 })).toBe(false);
  expect(compressionPathExcluded(config, { path: ".git", isDirectory: true, bytes: 0 })).toBe(false);
});

it("everything profile keeps generated files and cache profile keeps dependencies/builds", () => {
  const base = { source: "", outputFolder: "", mode: "fast" as const, excludedPaths: [], regexExclusions: [] };
  const entry = (path: string) => ({ path, isDirectory: true, bytes: 0 });
  expect(compressionPathExcluded({ ...base, ...compressionPresets[1] }, entry("node_modules"))).toBe(false);
  expect(compressionPathExcluded({ ...base, ...compressionPresets[2] }, entry("node_modules"))).toBe(false);
  expect(compressionPathExcluded({ ...base, ...compressionPresets[2] }, entry("build"))).toBe(false);
  expect(compressionPathExcluded({ ...base, ...compressionPresets[2] }, entry("cache"))).toBe(true);
});


it("renders presets and saved compression profiles in Vietnamese", () => {
  const configuration: AppConfiguration = { theme: "light", portableMode: false, createSafetyBackups: true, language: "vi", logLevel: "warn", codexEnvironments: [], fileTransferProfiles: [], compressionProfiles: [{ id: "custom", name: "My reusable project", config: { source: "", outputFolder: "", mode: "strong", excludedPaths: [], regexExclusions: [] } }] };
  const markup = renderToStaticMarkup(<I18nProvider value={{ language: "vi", t: (key, values) => translate("vi", key, values) }}><CompressionPage configuration={configuration} onSaveConfiguration={async () => {}} /></I18nProvider>);
  expect(markup).toContain("Project");
  expect(markup).toContain("Toàn bộ");
  expect(markup).toContain("Bỏ cache/log");
  expect(markup).toContain("Lưu profile");
  expect(markup).toContain("My reusable project");
});
