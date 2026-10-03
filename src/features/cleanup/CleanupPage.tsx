import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PageHeader } from "../../components/PageHeader";
import { useTranslation } from "../../i18n";
interface Scan { token: string; location: string; files: number; bytes: number; skipped: number; blocked: string | null }
interface Outcome { deleted: number; skipped: number; reclaimedBytes: number; errors: string[] }
export function CleanupPage() {
  const { t } = useTranslation();
  const [scan, setScan] = useState<Scan | null>(null); const [selected, setSelected] = useState(false); const [preview, setPreview] = useState(false);
  const [confirmation, setConfirmation] = useState(""); const [busy, setBusy] = useState(false); const [error, setError] = useState(""); const [result, setResult] = useState<Outcome | null>(null);
  const run = async (action: () => Promise<void>) => { setBusy(true); setError(""); try { await action(); } catch (e) { setError(String(e)); } finally { setBusy(false); } };
  return <><PageHeader title={t("cleanup.title")} description={t("cleanup.description")} action={<button disabled={busy} onClick={() => void run(async () => { setScan(null); setPreview(false); setConfirmation(""); setSelected(false); setResult(null); setScan(await invoke<Scan>("scan_cleanup")); })}>{t("cleanup.scan")}</button>} />
    <p className="notice-card">{t("cleanup.scope")}</p>
    {scan && <section className="backup-preview"><label className="cleanup-selection"><input type="checkbox" disabled={busy || !!scan.blocked || !scan.files} checked={selected} onChange={e => { setSelected(e.target.checked); setPreview(false); setConfirmation(""); }} /><span><strong>{t("cleanup.catalog")}</strong><small>{t("cleanup.selectForPreview")}</small></span></label><p><code>{scan.location}</code></p><p>{scan.files} {t("environment.files")} · {(scan.bytes / 1024 / 1024).toFixed(2)} MiB · {t("cleanup.skipped")}: {scan.skipped}</p><p>{t("cleanup.reason")}</p>{scan.blocked && <p className="error-banner">{scan.blocked}</p>}
      <button disabled={busy || !selected} onClick={() => setPreview(true)}>{t("environment.preview")}</button>
      {preview && <><p>{t("cleanup.confirmDescription")}</p><label>{t("cleanup.confirm")}<input disabled={busy} value={confirmation} onChange={e => setConfirmation(e.target.value)} /></label><button disabled={busy || confirmation !== "CLEAN" || !selected} onClick={() => void run(async () => { setResult(await invoke<Outcome>("execute_cleanup", { token: scan.token, confirmation })); setScan(null); setPreview(false); })}>{t("cleanup.execute")}</button></>}
    </section>}
    {error && <p role="alert" className="error-banner">{error}</p>}{result && <section className="notice-card" role="status"><p>{t("cleanup.result")}: {result.deleted} / {result.skipped} / {result.errors.length} · {result.reclaimedBytes} B</p>{result.errors.map((e,i) => <p key={i}>{e}</p>)}</section>}
  </>;
}
