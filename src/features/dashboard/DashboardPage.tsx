import { PageHeader } from "../../components/PageHeader";
import { useTranslation } from "../../i18n";
import type { BackupStorageOverview, BackupStorageSummary, DiagnosticsSnapshot } from "../../types/codex";

interface DashboardPageProps { diagnostics: DiagnosticsSnapshot | null; backupStorage: BackupStorageOverview | null; loading: boolean; error: string | null; onRefresh: () => void; }

const formatBytes = (bytes: number) => bytes < 1024 ? `${bytes} B` : bytes < 1024 * 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / (1024 * 1024)).toFixed(1)} MB`;

export function DashboardPage({ diagnostics, backupStorage, loading, error, onRefresh }: DashboardPageProps) {
  const { language, t } = useTranslation();
  const unifiedBackups = backupStorage?.sessionBackups.directory === backupStorage?.personalBundles.directory;
  const backupCount = backupStorage ? unifiedBackups ? backupStorage.sessionBackups.fileCount : backupStorage.sessionBackups.fileCount + backupStorage.personalBundles.fileCount : 0;
  const backupBytes = backupStorage ? unifiedBackups ? backupStorage.sessionBackups.totalBytes : backupStorage.sessionBackups.totalBytes + backupStorage.personalBundles.totalBytes : 0;
  const cards = [
    [t("dashboard.environment"), diagnostics?.codexHomeExists ? t("dashboard.detected") : t("dashboard.notDetected")],
    [t("dashboard.skills"), diagnostics ? String(diagnostics.skillsCount) : t("common.dash")],
    [t("dashboard.pets"), diagnostics ? String(diagnostics.petsCount) : t("common.dash")],
    [t("dashboard.backups"), backupStorage ? t("dashboard.backupSummary", { count: backupCount, bytes: formatBytes(backupBytes) }) : t("common.dash")]
  ];
  const features = [[t("dashboard.fileTransfer"), t("dashboard.fileTransferText")], [t("dashboard.compression"), t("dashboard.compressionText")], [t("dashboard.migration"), t("dashboard.migrationText")], [t("dashboard.localContent"), t("dashboard.localContentText")]];

  return (
    <>
      <PageHeader title={t("dashboard.title")} description={t("dashboard.description")} action={<button type="button" onClick={onRefresh} disabled={loading}>{loading ? t("common.working") : t("common.refresh")}</button>} />
      <section className="summary-grid" aria-label={t("dashboard.summary")}>
        {cards.map(([label, value]) => <article className="summary-card" key={label}><span>{label}</span><strong>{value}</strong></article>)}
      </section>
      <section className="dashboard-section">
        <h2>{t("dashboard.features")}</h2>
        <div className="dashboard-feature-grid">{features.map(([title, description]) => <article key={title}><h3>{title}</h3><p>{description}</p></article>)}</div>
      </section>
      <section className="dashboard-section">
        <h2>{t("dashboard.backupData")}</h2>
        <p>{t("dashboard.backupDataText")}</p>
        {error && <p role="alert" className="error-banner">{error}</p>}
        {backupStorage && <div className="dashboard-backup-grid"><BackupStorage title={unifiedBackups ? t("dashboard.backups") : t("dashboard.sessionBackups")} storage={backupStorage.sessionBackups} language={language} />{!unifiedBackups && <BackupStorage title={t("dashboard.personalBundles")} storage={backupStorage.personalBundles} language={language} />}</div>}
      </section>
      <section className="notice-card">
        <h2>{t("dashboard.safe")}</h2>
        <p>{t("dashboard.safeText")}</p>
      </section>
    </>
  );
}

function BackupStorage({ title, storage, language }: { title: string; storage: BackupStorageSummary; language: "en" | "vi" }) {
  const { t } = useTranslation();
  const date = new Intl.DateTimeFormat(language, { dateStyle: "short", timeStyle: "short" });
  return <article className="dashboard-backup-card"><h3>{title}</h3><p>{t("dashboard.backupSummary", { count: storage.fileCount, bytes: formatBytes(storage.totalBytes) })}</p><code>{storage.directory}</code>{storage.recentFiles.length === 0 ? <p>{t("dashboard.noBackups")}</p> : <><p>{t("dashboard.recentFiles")}</p><ul>{storage.recentFiles.map((file) => <li key={file.name}><strong>{file.name}</strong><span>{formatBytes(file.bytes)}{file.modifiedAt ? ` · ${date.format(new Date(file.modifiedAt * 1000))}` : ""}</span></li>)}</ul></>}</article>;
}
