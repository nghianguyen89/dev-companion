import { EnvironmentPage } from "../features/backup/EnvironmentPage";
import { CodexEnvironmentsPage } from "../features/codex-environments/CodexEnvironmentsPage";
import { FileTransferPageUx as FileTransferPage } from "../features/file-transfer/FileTransferPageUx";
import { CompressionPage } from "../features/compression/CompressionPage";
import { CleanupPage } from "../features/cleanup/CleanupPage";
import { useCallback, useState } from "react";
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
import { getConfiguration, getDiagnostics, saveConfiguration } from "../services/tauri";
import type { AppConfiguration } from "../types/codex";
import { I18nProvider, translate, type TranslationKey } from "../i18n";
import type { TranslationValues } from "../i18n";

type Page = "fileTransfer" | "compression" | "codexAccounts" | "environment" | "beyondCompare" | "sourceTree" | "xampp" | "cleanup" | "dashboard" | "conversations" | "backup" | "skills" | "pets" | "diagnostics" | "settings";
const navigation: Array<{ id: Page; label: TranslationKey; group: TranslationKey }> = [
  { id: "fileTransfer", label: "fileTransfer.title", group: "nav.tools" },
  { id: "compression", label: "compression.title", group: "nav.tools" },
  { id: "codexAccounts", label: "codexAccounts.title", group: "nav.codex" },
  { id: "environment", label: "environment.title", group: "nav.manage" }, { id: "cleanup", label: "cleanup.title", group: "nav.tools" },
  { id: "beyondCompare", label: "beyondCompare.title", group: "nav.manage" },
  { id: "sourceTree", label: "sourceTree.title", group: "nav.manage" },
  { id: "xampp", label: "xampp.title", group: "nav.manage" },
  { id: "dashboard", label: "nav.dashboard", group: "nav.overview" }, { id: "conversations", label: "nav.conversations", group: "nav.manage" },
  { id: "backup", label: "nav.backup", group: "nav.manage" }, { id: "skills", label: "nav.skills", group: "nav.codex" },
  { id: "pets", label: "nav.pets", group: "nav.codex" }, { id: "diagnostics", label: "nav.diagnostics", group: "nav.tools" }, { id: "settings", label: "nav.settings", group: "nav.preferences" }
];

export function App() {
  const [page, setPage] = useState<Page>("dashboard");
  const diagnostics = useAsyncValue(getDiagnostics);
  const configuration = useAsyncValue(getConfiguration);
  const { setValue: setConfiguration } = configuration;
  const save = useCallback(async (next: AppConfiguration) => { await saveConfiguration(next); setConfiguration(next); }, [setConfiguration]);
  const language = configuration.value?.language ?? "en";
  const t = (key: TranslationKey, values?: TranslationValues) => translate(language, key, values);
  const groups = [...new Set(navigation.map((item) => item.group))];

  return <I18nProvider value={{ language, t }}><main className="app-shell">
    <aside className="sidebar">
      <div className="brand"><span className="brand-mark">D</span><div><strong>Dev</strong><span>Companion</span></div></div>
      <nav aria-label={t("nav.primary")}>{groups.map((group) => <section key={group}><p>{t(group)}</p>{navigation.filter((item) => item.group === group).map((item) => <button key={item.id} className={page === item.id ? "active" : ""} type="button" onClick={() => setPage(item.id)}>{t(item.label)}</button>)}</section>)}</nav>
      <footer><span className="status-dot" /> {t("nav.localFoundation")}</footer>
    </aside>
    <section className="content">
      {page === "dashboard" && <DashboardPage diagnostics={diagnostics.value} />}
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
      {page === "settings" && <SettingsPage configuration={configuration.value} onSave={save} />}
    </section>
  </main></I18nProvider>;
}
