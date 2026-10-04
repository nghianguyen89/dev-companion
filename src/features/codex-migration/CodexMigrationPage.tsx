import { useEffect, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { useAsyncValue } from "../../hooks/useAsyncValue";
import { useTranslation } from "../../i18n";
import { createCodexMigration, deleteCodexMigrationArchive, getCodexMigrationOverview, inspectCodexMigration, listCodexMigrationArchives, openCodexMigrationArchive, previewCodexMigration, previewCodexMigrationRestore, restoreCodexMigration } from "../../services/tauri";
import type { CodexMigrationGroup, CodexMigrationPreview, CodexMigrationRestorePreview } from "../../types/codex";

const recommended: CodexMigrationGroup[] = ["chat", "settings", "skills", "pets"];
const components: CodexMigrationGroup[] = ["chat", "settings", "skills", "pets", "worktrees", "plugins", "visualizations"];
const size = (bytes: number) => bytes < 1024 * 1024 ? `${Math.ceil(bytes / 1024)} KiB` : `${(bytes / 1024 / 1024).toFixed(2)} MiB`;

export function CodexMigrationPage() {
  const { language, t } = useTranslation();
  const overview = useAsyncValue(getCodexMigrationOverview);
  const archives = useAsyncValue(listCodexMigrationArchives);
  const [accounts, setAccounts] = useState<string[]>([]);
  const [groups, setGroups] = useState<CodexMigrationGroup[]>(recommended);
  const [preview, setPreview] = useState<CodexMigrationPreview | null>(null);
  const [archive, setArchive] = useState<CodexMigrationPreview | null>(null);
  const [restore, setRestore] = useState<CodexMigrationRestorePreview | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const [busy, setBusy] = useState(false);
  const [backupStage, setBackupStage] = useState<"reviewing" | "creating" | null>(null);
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
  return <>
    <PageHeader title={t("codexMigration.title")} description={t("codexMigration.description")} action={<button type="button" disabled={busy} onClick={() => void overview.refresh()}>{t("common.refresh")}</button>} />
    <p className="notice-card">{t("codexMigration.boundary")}</p>
    <section className="backup-preview" aria-busy={overview.loading || busy}><h2>{t("codexMigration.accounts")}</h2>
      {overview.value?.accounts.length ? <div className="codex-migration-options">{overview.value.accounts.map((account) => <label key={account.id}><input type="checkbox" disabled={busy} checked={accounts.includes(account.id)} onChange={() => toggle(accounts, account.id, setAccounts)} /><strong>{account.label}</strong><code>{account.folder}</code></label>)}</div> : !overview.loading && <p>{t("codexMigration.noAccounts")}</p>}
      {overview.error && <p role="alert" className="error-banner">{overview.error}</p>}
    </section>
    <section className="backup-preview"><h2>{t("codexMigration.components")}</h2><div className="codex-migration-options">{estimates.map((estimate) => <label key={estimate.id}><input type="checkbox" disabled={busy} checked={groups.includes(estimate.id)} onChange={() => toggle(groups, estimate.id, setGroups)} /><span>{t(`codexMigration.${estimate.id}`)}</span><small>{estimate.files} {t("codexMigration.files")} · {size(estimate.bytes)}</small></label>)}</div><p><strong>{t("codexMigration.estimate")}:</strong> {size(estimatedTotal)}</p><button type="button" disabled={busy || accounts.length === 0 || groups.length === 0} onClick={backup}>{backupStage === "creating" ? t("codexMigration.creating") : backupStage ? t("codexMigration.scanning") : t("codexMigration.backup")}</button>{backupStage && <p role="status" className="migration-progress"><progress aria-label={t("codexMigration.progress")} />{backupStage === "creating" ? t("codexMigration.creating") : t("codexMigration.scanning")}</p>}{backupResult && <section role="status" className="success-card"><h2>{t("codexMigration.completed")}</h2><p>{backupResult}</p></section>}</section>
    {preview && <section className="backup-preview"><h2>{t("codexMigration.selected")}</h2><div className="migration-summary">{preview.groups.map((group) => <p key={group.id}><strong>{group.id}</strong> · {group.files} {t("codexMigration.files")} · {size(group.bytes)}</p>)}</div>{preview.excluded.length > 0 && <details><summary>{t("codexMigration.excluded")}</summary>{preview.excluded.map((group) => <p key={group.id}>{group.files} {t("codexMigration.files")} · {size(group.bytes)}<br />{group.reason}</p>)}</details>}<details><summary>{t("codexMigration.fileList")} ({preview.entries.length})</summary><ul className="safe-session-list">{preview.entries.map((entry) => <li key={entry.archivePath}><code>{entry.archivePath}</code> · {size(entry.bytes)}</li>)}</ul></details></section>}
    <section className="backup-preview" aria-busy={archives.loading}><div className="migration-archives-heading"><div><h2>{t("codexMigration.archives")}</h2>{archives.value && <p>{t("codexMigration.archiveSummary", { count: archives.value.archives.length, bytes: size(archives.value.totalBytes) })}</p>}</div><button type="button" disabled={busy || archives.loading} onClick={() => void archives.refresh()}>{t("common.refresh")}</button></div>{archives.loading && <p>{t("codexMigration.archiveLoading")}</p>}{archives.error && <p role="alert" className="error-banner">{archives.error}</p>}{archives.value?.archives.length === 0 && !archives.loading && <p>{t("codexMigration.archiveEmpty")}</p>}{archives.value?.archives.length ? <ul className="migration-archives">{archives.value.archives.map((item) => <li key={item.name}><div><strong>{item.name}</strong><small>{item.modifiedAt ? date.format(new Date(item.modifiedAt * 1000)) : t("common.dash")} · {size(item.bytes)}</small></div><div><button type="button" disabled={busy} onClick={() => void run(() => openCodexMigrationArchive(item.name))}>{t("codexMigration.openArchive")}</button><button type="button" className="danger-button" disabled={busy} onClick={() => { if (window.confirm(t("codexMigration.deleteArchiveConfirm", { name: item.name }))) void run(async () => { await deleteCodexMigrationArchive(item.name); await archives.refresh(); }); }}>{t("codexMigration.deleteArchive")}</button></div></li>)}</ul> : null}</section>
    <section className="backup-preview"><h2>{t("codexMigration.recovery")}</h2><button type="button" disabled={busy} onClick={() => void run(async () => { setArchive(await inspectCodexMigration()); setRestore(null); setConfirmation(""); })}>{t("codexMigration.inspect")}</button>{archive && <><p>{archive.entries.length} {t("codexMigration.files")} · SHA-256 ✓</p><button type="button" disabled={busy} onClick={() => void run(async () => { setRestore(await previewCodexMigrationRestore(archive.token)); setConfirmation(""); })}>{t("codexMigration.previewRestore")}</button></>}
      {restore && <><div className="conversation-table-wrap"><table className="conversation-table"><thead><tr><th>{t("codexMigration.accounts")}</th><th>{t("codexMigration.fileList")}</th><th>{t("codexMigration.status")}</th><th>MiB</th></tr></thead><tbody>{restore.items.map((item) => <tr key={item.archivePath}><td>{item.accountLabel}</td><td><code>{item.path}</code></td><td>{item.status}</td><td>{size(item.bytes)}</td></tr>)}</tbody></table></div><p>{t("codexMigration.statusHelp")}</p><label className="delete-confirmation">{t("codexMigration.confirm")}<input disabled={busy} value={confirmation} onChange={(event) => setConfirmation(event.target.value)} autoComplete="off" /></label><button type="button" disabled={busy || confirmation !== "RESTORE"} onClick={() => void run(async () => { const restored = await restoreCodexMigration(restore.token, confirmation); setResult(t("codexMigration.result", { restored: restored.restored, skipped: restored.skipped })); if (restored.errors.length) setError(restored.errors.join("\n")); setRestore(null); setConfirmation(""); })}>{t("codexMigration.restore")}</button></>}
    </section>
    {error && <p role="alert" className="error-banner">{error}</p>}{result && <p role="status" className="notice-card">{result}</p>}
  </>;
}
