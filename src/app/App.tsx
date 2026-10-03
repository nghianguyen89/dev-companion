import { EnvironmentPage } from "../features/backup/EnvironmentPage";
import { CodexEnvironmentsPage } from "../features/codex-environments/CodexEnvironmentsPage";
import { FileTransferPageUx as FileTransferPage } from "../features/file-transfer/FileTransferPageUx";
import { CompressionPage } from "../features/compression/CompressionPage";
import { CleanupPage } from "../features/cleanup/CleanupPage";
import { useCallback, useEffect, useState } from "react";
import { BackupPage } from "../features/backup/BackupPage";
import { BeyondComparePage } from "../features/backup/BeyondComparePage";
import { SourceTreePage } from "../features/backup/SourceTreePage";
import { XamppPage } from "../features/backup/XamppPage";
import { ConversationsPage } from "../features/conversations/ConversationsPage";
import { DashboardPage } from "../features/dashboard/DashboardPage";
import { DiagnosticsPage } from "../features/diagnostics/DiagnosticsPage";
import { PetsPage } from "../features/pets/PetsPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { SkillsPage } from "../features/skills/SkillsPage";
import { useAsyncValue } from "../hooks/useAsyncValue";
import { getBackupStorage, getConfiguration, getDiagnostics, isTauriRuntime, saveConfiguration } from "../services/tauri";
import type { AppConfiguration } from "../types/codex";
import { I18nProvider, translate, type TranslationKey, type TranslationValues } from "../i18n";
import { resolveTheme } from "./theme";

type Page = "fileTransfer" | "compression" | "codexAccounts" | "environment" | "beyondCompare" | "sourceTree" | "xampp" | "cleanup" | "dashboard" | "conversations" | "backup" | "skills" | "pets" | "diagnostics" | "settings";
const navigation: Array<{ id: Page; label: TranslationKey; group: TranslationKey }> = [
  { id: "dashboard", label: "nav.dashboard", group: "nav.overview" },
  { id: "fileTransfer", label: "fileTransfer.title", group: "nav.tools" },
  { id: "compression", label: "compression.title", group: "nav.tools" },
  { id: "codexAccounts", label: "codexAccounts.title", group: "nav.codex" },
  { id: "environment", label: "environment.title", group: "nav.manage" }, { id: "cleanup", label: "cleanup.title", group: "nav.tools" },
  { id: "beyondCompare", label: "beyondCompare.title", group: "nav.manage" },
  { id: "sourceTree", label: "sourceTree.title", group: "nav.manage" },
  { id: "xampp", label: "xampp.title", group: "nav.manage" },
  { id: "conversations", label: "nav.conversations", group: "nav.manage" },
  { id: "backup", label: "nav.backup", group: "nav.manage" }, { id: "skills", label: "nav.skills", group: "nav.codex" },
  { id: "pets", label: "nav.pets", group: "nav.codex" }, { id: "diagnostics", label: "nav.diagnostics", group: "nav.tools" }, { id: "settings", label: "nav.settings", group: "nav.preferences" }
];
const navIconPaths: Record<Page, string> = {
  fileTransfer: "M4 7h11m-4-4 4 4-4 4M20 17H9m4-4-4 4 4 4",
  compression: "M4 5h16v14H4zM4 9h16m-11 4h6m-4 3h2",
  cleanup: "M3 6h18M8 6V4h8v2m-9 0 1 14h8l1-14m-8 4v6m4-6v6",
  diagnostics: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm0-10v5m0-8h.01",
  codexAccounts: "M20 21a8 8 0 0 0-16 0m8-10a4 4 0 1 0 0-8 4 4 0 0 0 0 8",
  environment: "M4 5h16v14H4zM7 9l3 3-3 3m5 0h5",
  beyondCompare: "M4 5h16v14H4zM12 5v14M7 9h2m4 0h4M7 14h3m3 0h4",
  sourceTree: "M6 3v12a3 3 0 0 0 6 0V8a3 3 0 0 1 6 0v1M6 7h.01M12 15h.01M18 9h.01",
  xampp: "M4 4h16v6H4zm0 10h16v6H4zm4-7v.01m0 10v.01",
  conversations: "M5 4h14v11H9l-4 4V4Zm4 5h6",
  dashboard: "M4 4h6v6H4zm10 0h6v6h-6zM4 14h6v6H4zm10 0h6v6h-6z",
  backup: "M5 4h14v16H5zM8 4v5h8V4m-8 9h8m-8 3h5",
  skills: "m12 3 1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8L12 3Zm7 13 .7 2.3L22 19l-2.3.7L19 22l-.7-2.3L16 19l2.3-.7L19 16Z",
  pets: "M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10Z",
  settings: "M4 7h16M8 7v.01M4 12h16m-5 0v.01M4 17h16m-11 0v.01",
};
function NavIcon({ page }: { page: Page }) { return <svg className="nav-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round"><path d={navIconPaths[page]} /></svg>; }

export function App() {
  const browserPreview = !isTauriRuntime();
  const [page, setPage] = useState<Page>("dashboard");
  const [systemPrefersDark, setSystemPrefersDark] = useState(() => typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => setSystemPrefersDark(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  const diagnostics = useAsyncValue(getDiagnostics);
  const backupStorage = useAsyncValue(getBackupStorage);
  const configuration = useAsyncValue(getConfiguration);
  const { setValue: setConfiguration } = configuration;
  const save = useCallback(async (next: AppConfiguration) => { await saveConfiguration(next); setConfiguration(next); }, [setConfiguration]);
  const language = configuration.value?.language ?? "en";
  const theme = resolveTheme(configuration.value?.theme, systemPrefersDark);
  const t = (key: TranslationKey, values?: TranslationValues) => translate(language, key, values);
  const groups = [...new Set(navigation.map((item) => item.group))];

  return <I18nProvider value={{ language, t }}><main className="app-shell" data-theme={theme}>
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark">D</span><div><strong>Dev</strong><span>Companion</span></div></div>
      <nav aria-label={t("nav.primary")}>{groups.map((group) => <section key={group}><p>{t(group)}</p>{navigation.filter((item) => item.group === group).map((item) => <button key={item.id} className={page === item.id ? "active" : ""} type="button" onClick={() => setPage(item.id)}><NavIcon page={item.id} />{t(item.label)}</button>)}</section>)}</nav>
      <footer><span className="status-dot" /> {t("nav.localFoundation")}</footer>
    </aside>
    <section className="content">
      {browserPreview && <p className="notice-card browser-preview">{t("common.browserPreview")}</p>}
      {page === "dashboard" && <DashboardPage diagnostics={diagnostics.value} backupStorage={backupStorage.value} loading={diagnostics.loading || backupStorage.loading} error={backupStorage.error} onRefresh={() => { void Promise.all([diagnostics.refresh(), backupStorage.refresh()]); }} />}
      {page === "conversations" && <ConversationsPage />}
      {page === "backup" && <BackupPage />}
      {page === "environment" && <EnvironmentPage />}
      {page === "codexAccounts" && <CodexEnvironmentsPage />}
      {page === "beyondCompare" && <BeyondComparePage />}
      {page === "sourceTree" && <SourceTreePage />}
      {page === "xampp" && <XamppPage />}
      {page === "cleanup" && <CleanupPage />}
      {page === "fileTransfer" && configuration.value && <FileTransferPage configuration={configuration.value} onSaveConfiguration={save} />}
      {page === "compression" && <CompressionPage />}
      {page === "skills" && <SkillsPage />}
      {page === "pets" && <PetsPage />}
      {page === "diagnostics" && <DiagnosticsPage diagnostics={diagnostics.value} loading={diagnostics.loading} error={diagnostics.error} onRefresh={diagnostics.refresh} />}
      {page === "settings" && <SettingsPage configuration={configuration.value} loadError={configuration.error} onSave={save} />}
    </section>
  </main></I18nProvider>;
}
