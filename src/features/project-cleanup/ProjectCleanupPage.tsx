import { useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { useTranslation } from "../../i18n";
import { cancelProjectCleanup, executeProjectCleanup, pickProjectCleanupFolder, previewProjectCleanup, scanProjectCleanup } from "../../services/tauri";
import type { ProjectCleanupCandidate, ProjectCleanupMode, ProjectCleanupOutcome, ProjectCleanupPreview, ProjectCleanupScan } from "../../types/codex";

const size = (bytes: number) => (bytes / 1024 / 1024).toFixed(2) + " MiB";

export function ProjectCleanupCandidates({ candidates, selected, disabled, onToggle }: {
  candidates: ProjectCleanupCandidate[]; selected: string[]; disabled: boolean; onToggle: (id: string) => void;
}) {
  const { t } = useTranslation();
  return <div className="conversation-table-wrap"><table className="conversation-table"><thead><tr><th>{t("projectCleanup.select")}</th><th>{t("projectCleanup.path")}</th><th>{t("projectCleanup.reason")}</th><th>{t("projectCleanup.size")}</th></tr></thead><tbody>{candidates.map(item => <tr key={item.id}>
    <td><input type="checkbox" aria-label={item.path} disabled={disabled || !!item.blocked} checked={selected.includes(item.id)} onChange={() => onToggle(item.id)} /></td>
    <td><code>{item.path}</code><small>{item.files} {t("environment.files")}</small></td>
    <td>{t(item.kind === "nodeDependencies" ? "projectCleanup.nodeDependencies" : item.kind === "pythonCache" ? "projectCleanup.pythonCache" : "projectCleanup.nodeCache")}{item.blocked && <p role="status">{t("projectCleanup.blocked")}: {item.blocked}</p>}</td>
    <td>{item.blocked ? "≥ " : ""}{size(item.bytes)}</td>
  </tr>)}</tbody></table></div>;
}

export function ProjectCleanupPage({ onArchive }: { onArchive?: (source: string) => void } = {}) {
  const { t } = useTranslation();
  const [root, setRoot] = useState("");
  const [scan, setScan] = useState<ProjectCleanupScan | null>(null);
  const [selected, setSelected] = useState<string[]>([]);
  const [preview, setPreview] = useState<ProjectCleanupPreview | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const [result, setResult] = useState<(ProjectCleanupOutcome & { source: string }) | null>(null);
  const [mode, setMode] = useState<ProjectCleanupMode>("recycle");
  const [phase, setPhase] = useState<"idle" | "pick" | "scan" | "preview" | "delete">("idle");
  const [error, setError] = useState("");
  const [visible, setVisible] = useState(100);
  const busy = phase !== "idle";
  const expectedConfirmation = mode === "permanent" ? "DELETE GENERATED" : "RECYCLE GENERATED";
  const invalidatePreview = () => { setPreview(null); setConfirmation(""); setResult(null); };
  const changeRoot = (next: string) => { setRoot(next); setScan(null); setSelected([]); setVisible(100); invalidatePreview(); };
  const changeSelection = (next: string[]) => { setSelected(next); invalidatePreview(); };
  const run = async (next: typeof phase, action: () => Promise<void>) => {
    setPhase(next); setError("");
    try { await action(); } catch (e) { setError(String(e)); } finally { setPhase("idle"); }
  };
  const scanRoot = () => void run("scan", async () => {
    setScan(null); setSelected([]); setVisible(100); invalidatePreview();
    setScan(await scanProjectCleanup(root));
  });
  const eligible = scan?.candidates.filter(item => !item.blocked) ?? [];
  return <div className="project-cleanup"><PageHeader title={t("projectCleanup.title")} description={t("projectCleanup.description")} />
    <p className="notice-card">{t("projectCleanup.boundary")}</p>
    <section className="backup-preview"><label className="project-cleanup-source">{t("projectCleanup.source")}<input disabled={busy} value={root} onChange={event => changeRoot(event.target.value)} placeholder={t("projectCleanup.sourceHint")} /></label>
      <label className="project-cleanup-source">{t("projectCleanup.deleteMode")}<select disabled={busy} value={mode} onChange={event => { setMode(event.target.value as ProjectCleanupMode); invalidatePreview(); }}><option value="recycle">{t("projectCleanup.recycle")}</option><option value="permanent">{t("projectCleanup.permanentMode")}</option></select></label>
      <div className="project-cleanup-actions"><button disabled={busy} onClick={() => void run("pick", async () => { const folder = await pickProjectCleanupFolder(t("projectCleanup.choose")); if (folder) changeRoot(folder); })}>{t("projectCleanup.choose")}</button>
        <button disabled={busy || !root.trim()} onClick={scanRoot}>{t("cleanup.scan")}</button>
        {phase === "scan" && <button onClick={() => void cancelProjectCleanup().catch(e => setError(String(e)))}>{t("compression.cancel")}</button>}
      </div>
      {busy && <p role="status" aria-live="polite">{t(phase === "scan" ? "projectCleanup.scanning" : phase === "preview" ? "projectCleanup.previewing" : phase === "delete" ? "projectCleanup.deleting" : "projectCleanup.picking")}</p>}
    </section>
    {scan && <section className="backup-preview"><h2>{t("projectCleanup.found")}</h2><p><code>{scan.root}</code></p><p>{t("projectCleanup.scanSummary", { count: scan.candidates.length, skipped: scan.skipped })}</p><p>{t("projectCleanup.sizeHelp")}</p>
      {scan.incomplete && <p role="alert" className="error-banner">{t("projectCleanup.incomplete")}</p>}
      {scan.candidates.length === 0 ? <p>{t("projectCleanup.empty")}</p> : <>
        <div className="project-cleanup-actions"><button disabled={busy || scan.incomplete || !eligible.length} onClick={() => changeSelection(eligible.map(item => item.id))}>{t("projectCleanup.selectAll")}</button><button disabled={busy || !selected.length} onClick={() => changeSelection([])}>{t("fileTransfer.clear")}</button>
          <button disabled={busy || scan.incomplete || !selected.length} onClick={() => void run("preview", async () => { setPreview(null); setConfirmation(""); setResult(null); setPreview(await previewProjectCleanup(scan.token, selected, mode)); })}>{t("environment.preview")}</button><span>{t("projectCleanup.selected", { count: selected.length })}</span>
        </div>
        <ProjectCleanupCandidates candidates={scan.candidates.slice(0, visible)} selected={selected} disabled={busy || scan.incomplete} onToggle={id => changeSelection(selected.includes(id) ? selected.filter(value => value !== id) : [...selected, id])} />
        {visible < scan.candidates.length && <button disabled={busy} onClick={() => setVisible(value => value + 100)}>{t("projectCleanup.showMore")}</button>}
      </>}
    </section>}
    {preview && <section className="backup-preview"><h2>{t("projectCleanup.review")}</h2><p>{t("projectCleanup.previewSummary", { count: preview.candidates.length, files: preview.files, size: size(preview.bytes) })}</p>
      <p className={mode === "permanent" ? "error-banner" : "notice-card"}>{t(mode === "permanent" ? "projectCleanup.permanent" : "projectCleanup.recycleHelp")}</p>
      <div className="project-cleanup-actions"><label>{t(mode === "permanent" ? "projectCleanup.confirm" : "projectCleanup.recycleConfirm")}<input disabled={busy} value={confirmation} onChange={event => setConfirmation(event.target.value)} autoComplete="off" /></label>
        <button disabled={busy || confirmation !== expectedConfirmation} onClick={() => void run("delete", async () => {
          const token = preview.token; setPreview(null); setConfirmation(""); setScan(null); setSelected([]);
          setResult({ ...await executeProjectCleanup(token, confirmation), source: preview.root });
        })}>{t(mode === "permanent" ? "projectCleanup.execute" : "projectCleanup.recycleExecute")}</button>
      </div><ul className="project-cleanup-preview-paths">{preview.candidates.map(item => <li key={item.id}><code>{item.path}</code> · {size(item.bytes)}</li>)}</ul>
    </section>}
    {error && <p role="alert" className="error-banner">{error}</p>}
    {result && <section className="notice-card" role="status"><p>{t(result.mode === "recycle" ? "projectCleanup.recycleResult" : "projectCleanup.result", { files: result.mode === "recycle" ? result.recycledFiles : result.deletedFiles, folders: result.mode === "recycle" ? result.recycledDirectories : result.removedDirectories, size: size(result.mode === "recycle" ? result.recycledBytes : result.reclaimedBytes), errors: result.errors.length })}</p>{result.errors.map((message, index) => <p key={index} className="error-banner">{message}</p>)}<p>{t("projectCleanup.nextStep")}</p>{onArchive && <button disabled={busy} onClick={() => onArchive(result.source)}>{t("projectCleanup.archive")}</button>}</section>}
  </div>;
}
