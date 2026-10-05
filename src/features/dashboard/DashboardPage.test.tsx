import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { DashboardPage } from "./DashboardPage";

it("shows a unified backup inventory once both archive types share a folder", () => {
  const html = renderToStaticMarkup(<DashboardPage diagnostics={{ operatingSystem: "windows", architecture: "x86_64", codexHome: "C:\\Users\\me\\.codex", codexHomeExists: true, configDir: "config", backupDir: "backups", codexCliVersion: null, skillsCount: 2, petsCount: 1 }} backupStorage={{ sessionBackups: { directory: "backups", fileCount: 1, totalBytes: 3, recentFiles: [{ name: "sessions.zip", bytes: 3, modifiedAt: null }] }, personalBundles: { directory: "backups", fileCount: 1, totalBytes: 3, recentFiles: [{ name: "sessions.zip", bytes: 3, modifiedAt: null }] } }} loading={false} error={null} onRefresh={() => {}} />);
  expect(html).toContain("Existing backup data");
  expect(html).toContain("sessions.zip");
  expect(html).toContain("Available tools");
});
