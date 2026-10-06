import { useEffect, useMemo, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { useAsyncValue } from "../../hooks/useAsyncValue";
import { useTranslation, type TranslationKey } from "../../i18n";
import { createCodexMigration, deleteCodexMigrationArchive, getCodexMigrationOverview, inspectCodexMigrationArchive, listCodexMigrationArchives, openCodexMigrationArchive, previewCodexMigration, previewCodexMigrationRestore, restoreCodexMigration, stopCodexProcesses } from "../../services/tauri";
import type { CodexMigrationGroup, CodexMigrationPreview, CodexMigrationRestorePreview, CodexMigrationRestoreProgress } from "../../types/codex";

const components: CodexMigrationGroup[] = ["chat", "settings", "skills", "pets", "projects", "state", "worktrees", "plugins", "visualizations"];
const recommended: CodexMigrationGroup[] = components;
const size = (bytes: number) => bytes < 1024 * 1024 ? `${Math.ceil(bytes / 1024)} KiB` : `${(bytes / 1024 / 1024).toFixed(2)} MiB`;
const restoreProgressKeys: Record<CodexMigrationRestoreProgress["stage"], TranslationKey> = {
  queued: "migrationRestore.progressQueued", "checking-processes": "migrationRestore.progressCheckingProcesses", "archive-validation": "migrationRestore.progressArchiveValidation", "destination-check": "migrationRestore.progressDestinationCheck", "safety-scan": "migrationRestore.progressSafetyScan", "safety-backup": "migrationRestore.progressSafetyBackup", "safety-validation": "migrationRestore.progressSafetyValidation", removing: "migrationRestore.progressRemoving", restoring: "migrationRestore.progressRestoring", rollback: "migrationRestore.progressRollback", completed: "migrationRestore.progressCompleted",
};
const restoreOperationKeys: Record<NonNullable<CodexMigrationRestoreProgress["operation"]>, TranslationKey> = {
  "prepare-target": "migrationRestore.operationPrepareTarget", "open-target": "migrationRestore.operationOpenTarget", "read-archive": "migrationRestore.operationReadArchive", "copy-file": "migrationRestore.operationCopyFile", "flush-file": "migrationRestore.operationFlushFile", "close-file": "migrationRestore.operationCloseFile", "verify-file": "migrationRestore.operationVerifyFile", "verify-size": "migrationRestore.operationVerifySize", "verify-seek": "migrationRestore.operationVerifySeek", "verify-read": "migrationRestore.operationVerifyRead", "verify-digest": "migrationRestore.operationVerifyDigest", "check-target": "migrationRestore.operationCheckTarget", "create-parent": "migrationRestore.operationCreateParent", "recheck-target": "migrationRestore.operationRecheckTarget", "copy-read": "migrationRestore.operationCopyRead", "copy-write": "migrationRestore.operationCopyWrite",
};

export function CodexMigrationPage() {
  const { language, t } = useTranslation();
  const overview = useAsyncValue(getCodexMigrationOverview);
  const archives = useAsyncValue(listCodexMigrationArchives);
  const [accounts, setAccounts] = useState<string[]>([]);
  const [groups, setGroups] = useState<CodexMigrationGroup[]>(recommended);
  const [preview, setPreview] = useState<CodexMigrationPreview | null>(null);
  const [archive, setArchive] = useState<CodexMigrationPreview | null>(null);
  const [selectedArchive, setSelectedArchive] = useState<string | null>(null);
  const [restore, setRestore] = useState<CodexMigrationRestorePreview | null>(null);
  const [restoreProgress, setRestoreProgress] = useState<CodexMigrationRestoreProgress | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState(false);
  const [backupStage, setBackupStage] = useState<"reviewing" | "creating" | null>(null);
  const [restoreStage, setRestoreStage] = useState<"reading" | "preparing" | "restoring" | null>(null);
  const [backupResult, setBackupResult] = useState("");
  const [error, setError] = useState("");
  const [result, setResult] = useState("");
  useEffect(() => { if (overview.value) setAccounts((current) => current.length ? current : overview.value!.accounts.map((account) => account.id)); }, [overview.value]);
  const estimates = components.map((id) => {
    const totals = (overview.value?.estimates ?? []).filter((estimate) => accounts.includes(estimate.accountId)).map((estimate) => estimate.groups.find((group) => group.id === id));
    return { id, files: totals.reduce((total, group) => total + (group?.files ?? 0), 0), bytes: totals.reduce((total, group) => total + (group?.bytes ?? 0), 0) };
  });
  const estimatedTotal = estimates.filter((estimate) => groups.includes(estimate.id)).reduce((total, estimate) => total + estimate.bytes, 0);
  const date = new Intl.DateTimeFormat(language, { dateStyle: "short", timeStyle: "short" });
  const run = async (action: () => Promise<void>) => { setBusy(true); setError(""); setBackupResult(""); setResult(""); try { await action(); } catch (reason) { setError(typeof reason === "string" ? reason : t("codexMigration.error")); } finally { setBusy(false); } };
  const toggle = <T,>(values: T[], value: T, set: (next: T[]) => void) => { set(values.includes(value) ? values.filter((item) => item !== value) : [...values, value]); setPreview(null); };
  const backup = () => void run(async () => {
    setBackupStage("reviewing");
    try {
      const next = await previewCodexMigration(accounts, groups);
      setPreview(next);
      if (next.entries.length === 0) return;
      setBackupStage("creating");
      const created = await createCodexMigration(next.token);
      setBackupResult(`${t("codexMigration.created")}: ${created.archivePath} · ${created.files} ${t("codexMigration.files")} · ${size(created.archiveBytes)}`);
      await archives.refresh();
    } finally {
      setBackupStage(null);
    }
  });
  const inspectArchive = (name: string) => void run(async () => {
    setRestoreStage("reading");
    setRestoreProgress(null);
    try {
      setArchive(await inspectCodexMigrationArchive(name));
      setSelectedArchive(name);
      setRestore(null);
      setConfirmation("");
    } finally {
      setRestoreStage(null);
    }
  });
  const previewRestore = () => void run(async () => {
    if (!archive) return;
    setRestoreStage("preparing");
    setRestoreProgress(null);
    try {
      setRestore(await previewCodexMigrationRestore(archive.token));
      setConfirmation("");
    } finally {
      setRestoreStage(null);
    }
  });
  const restoreMigration = () => {
    if (!restore) return;
    setRestoreStage("restoring");
    setRestoreProgress({ stage: "queued", completed: 0, total: 0 });
    void run(async () => {
      try {
        const restored = await restoreCodexMigration(restore.token, confirmation, true, setRestoreProgress);
        let message = t("codexMigration.result", { restored: restored.restored, skipped: restored.skipped });
        if (restored.safetyArchive) message += `\n${t("migrationRestore.safety", { path: restored.safetyArchive })}`;
        setResult(message);
        const errors = [...restored.errors];
        if (restored.rollbackRemaining) errors.push(t("migrationRecovery.incomplete", { count: restored.rollbackRemaining }));
        if (errors.length) setError(errors.join("\n"));
        if (restored.errors.length === 0 && restored.rollbackRemaining === 0) setRestoreProgress({ stage: "completed", completed: restored.restored + restored.skipped, total: restore.items.length });
        else setRestoreProgress(null);
        setRestore(null);
        setConfirmation("");
      } catch (reason) {
        setRestoreProgress(null);
        throw reason;
      } finally {
        setRestoreStage(null);
      }
    });
  };
  const restoreCounts = restore?.items.reduce((counts, item) => ({ ...counts, [item.status]: counts[item.status] + 1 }), { new: 0, identical: 0, conflict: 0 });
  const restoreProgressStatus = restoreProgress && (restoreStage === "restoring" || restoreProgress.stage === "completed") ? <p role="status" aria-live="polite" aria-busy={restoreStage === "restoring"} className="migration-progress">{restoreStage === "restoring" && restoreProgress.stage !== "completed" && (restoreProgress.total > 0 ? <progress value={Math.min(restoreProgress.completed, restoreProgress.total)} max={restoreProgress.total} aria-label={t(restoreProgressKeys[restoreProgress.stage])} /> : <progress aria-label={t(restoreProgressKeys[restoreProgress.stage])} />)}<span>{t(restoreProgressKeys[restoreProgress.stage])}{restoreProgress.total > 0 && ` · ${t("migrationRestore.progressFiles", { completed: restoreProgress.completed, total: restoreProgress.total })}`}{restoreProgress.stage === "safety-scan" && restoreProgress.total === 0 && restoreProgress.completed > 0 && ` · ${t("migrationRestore.progressScannedFiles", { completed: restoreProgress.completed })}`}</span></p> : null;
  const restoreProgressDetails = restoreProgress && (restoreStage === "restoring" || restoreProgress.stage === "completed") && (restoreProgress.file || restoreProgress.operation || restoreProgress.bytesCopied != null) ? <p className="migration-progress-details">{restoreProgress.operation && <span>{t(restoreOperationKeys[restoreProgress.operation])}</span>}{restoreProgress.file && <code>{t("migrationRestore.currentFile", { file: restoreProgress.file })}</code>}{restoreProgress.bytesCopied != null && restoreProgress.fileBytes != null && <small>{t("migrationRestore.progressBytes", { bytesCopied: restoreProgress.bytesCopied, fileBytes: restoreProgress.fileBytes })}</small>}</p> : null;
  const restoreTable = useMemo(() => restore && <div className="conversation-table-wrap migration-restore-table"><table className="conversation-table"><thead><tr><th>{t("codexMigration.accounts")}</th><th>{t("codexMigration.fileList")}</th><th>{t("codexMigration.status")}</th><th>MiB</th></tr></thead><tbody>{restore.items.map((entry) => <tr key={entry.archivePath}><td>{entry.accountLabel}</td><td><code>{entry.path}</code></td><td>{entry.status}</td><td>{size(entry.bytes)}</td></tr>)}</tbody></table></div>, [restore, t]);
  const expectedConfirmation = "REPLACE CODEX";
  return <>
    <PageHeader title={t("codexMigration.title")} description={t("codexMigration.description")} action={<span className="page-actions"><button type="button" disabled={busy} onClick={() => { if (window.confirm("Dừng toàn bộ Codex Desktop/CLI còn chạy? Các tác vụ Codex đang làm sẽ bị hủy.")) void run(async () => { const stopped = await stopCodexProcesses(); setResult(`Đã dừng ${stopped.terminated} tiến trình Codex.`); await overview.refresh(); }); }}>Đóng tiến trình Codex</button><button type="button" disabled={busy} onClick={() => void overview.refresh()}>{t("common.refresh")}</button></span>} />
    <p className="notice-card">{t("codexMigration.boundary")}</p>
    <section className="backup-preview" aria-busy={overview.loading || busy}><h2>{t("codexMigration.accounts")}</h2>
      {overview.value?.accounts.length ? <div className="codex-migration-options">{overview.value.accounts.map((account) => <label key={account.id}><input type="checkbox" disabled={busy} checked={accounts.includes(account.id)} onChange={() => toggle(accounts, account.id, setAccounts)} /><strong>{account.label}</strong><code>{account.folder}</code></label>)}</div> : !overview.loading && <p>{t("codexMigration.noAccounts")}</p>}
      {overview.error && <p role="alert" className="error-banner">{overview.error}</p>}
    </section>
    <section className="backup-preview"><h2>{t("codexMigration.components")}</h2><div className="codex-migration-options">{estimates.map((estimate) => <label key={estimate.id}><input type="checkbox" disabled={busy} checked={groups.includes(estimate.id)} onChange={() => toggle(groups, estimate.id, setGroups)} /><span>{t(`codexMigration.${estimate.id}`)}</span><small>{estimate.files} {t("codexMigration.files")} · {size(estimate.bytes)}</small></label>)}</div><p><strong>{t("codexMigration.estimate")}:</strong> {size(estimatedTotal)}</p><button type="button" disabled={busy || accounts.length === 0 || groups.length === 0} onClick={backup}>{backupStage === "creating" ? t("codexMigration.creating") : backupStage ? t("codexMigration.scanning") : t("codexMigration.backup")}</button>{backupStage && <p role="status" className="migration-progress"><progress aria-label={t("codexMigration.progress")} />{backupStage === "creating" ? t("codexMigration.creating") : t("codexMigration.scanning")}</p>}{backupResult && <section role="status" className="success-card"><h2>{t("codexMigration.completed")}</h2><p>{backupResult}</p></section>}</section>
    {preview && <section className="backup-preview"><h2>{t("codexMigration.selected")}</h2><div className="migration-summary">{preview.groups.map((group) => <p key={group.id}><strong>{group.id}</strong> · {group.files} {t("codexMigration.files")} · {size(group.bytes)}</p>)}</div>{preview.excluded.length > 0 && <details><summary>{t("codexMigration.excluded")}</summary>{preview.excluded.map((group) => <p key={group.id}>{group.files} {t("codexMigration.files")} · {size(group.bytes)}<br />{group.reason}</p>)}</details>}<details><summary>{t("codexMigration.fileList")} ({preview.entries.length})</summary><ul className="safe-session-list">{preview.entries.map((entry) => <li key={entry.archivePath}><code>{entry.archivePath}</code> · {size(entry.bytes)}</li>)}</ul></details></section>}
    <section className="backup-preview" aria-busy={archives.loading}><div className="migration-archives-heading"><div><h2>{t("codexMigration.archives")}</h2>{archives.value && <p>{t("codexMigration.archiveSummary", { count: archives.value.archives.length, bytes: size(archives.value.totalBytes) })}</p>}</div><button type="button" disabled={busy || archives.loading} onClick={() => void archives.refresh()}>{t("common.refresh")}</button></div>{archives.loading && <p>{t("codexMigration.archiveLoading")}</p>}{archives.error && <p role="alert" className="error-banner">{archives.error}</p>}{archives.value?.archives.length === 0 && !archives.loading && <p>{t("codexMigration.archiveEmpty")}</p>}{archives.value?.archives.length ? <ul className="migration-archives">{archives.value.archives.map((item) => <li key={item.name}><div><strong>{item.name}</strong><small>{item.modifiedAt ? date.format(new Date(item.modifiedAt * 1000)) : t("common.dash")} · {size(item.bytes)}</small></div><div><button type="button" disabled={busy} onClick={() => inspectArchive(item.name)}>{t("codexMigration.restore")}</button><button type="button" disabled={busy} onClick={() => void run(() => openCodexMigrationArchive(item.name))}>{t("codexMigration.openArchive")}</button><button type="button" className="danger-button" disabled={busy} onClick={() => { if (window.confirm(t("codexMigration.deleteArchiveConfirm", { name: item.name }))) void run(async () => { await deleteCodexMigrationArchive(item.name); if (selectedArchive === item.name) { setSelectedArchive(null); setArchive(null); setRestore(null); } await archives.refresh(); }); }}>{t("codexMigration.deleteArchive")}</button></div>{selectedArchive === item.name && archive && <div className="source-tree-recovery" aria-busy={busy && Boolean(restoreStage)}>{restoreStage === "reading" && <p role="status" className="migration-progress"><progress aria-label={t("migrationRestore.reading")} />{t("migrationRestore.reading")}</p>}<p>{archive.entries.length} {t("codexMigration.files")} · SHA-256 ✓</p><div className="migration-summary">{archive.groups.map((group) => <p key={group.id}><strong>{group.id}</strong> · {group.files} {t("codexMigration.files")} · {size(group.bytes)}</p>)}</div><button type="button" disabled={busy} onClick={previewRestore}>{restoreStage === "preparing" ? t("migrationRestore.preparing") : t("codexMigration.previewRestore")}</button>{restoreStage === "preparing" && <p role="status" className="migration-progress"><progress aria-label={t("migrationRestore.preparing")} />{t("migrationRestore.preparing")}</p>}{restore && <><p>{t("migrationRestore.counts", restoreCounts)}</p><p className="partial-card">{t("migrationRestore.warning")}</p><p>{t("migrationRestore.replaceDescription")}</p><label className="delete-confirmation">{t("migrationRestore.confirmReplace")}<input disabled={busy} value={confirmation} onChange={(event) => setConfirmation(event.target.value)} autoComplete="off" /></label><button type="button" disabled={busy || confirmation !== expectedConfirmation} onClick={restoreMigration}>{t("migrationRestore.replaceAction")}</button>{restoreStage === "restoring" && restoreProgressStatus}{restoreStage === "restoring" && restoreProgressDetails}{restoreTable}</>}{restoreStage !== "restoring" && restoreProgress?.stage === "completed" && <>{restoreProgressStatus}{restoreProgressDetails}</>}</div>}</li>)}</ul> : null}</section>
    {error && <p role="alert" className="error-banner">{error}</p>}{result && <p role="status" className="notice-card">{result}</p>}
  </>;
}
