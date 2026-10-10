import { useState } from "react";
import { XamppDomains } from "./XamppDomains";
import { PageHeader } from "../../components/PageHeader";
import { useTranslation } from "../../i18n";
import { createXamppBundle, getXamppReadiness, inspectXamppBundle, previewXampp, previewXamppRecovery, recoverXampp } from "../../services/tauri";
import type { XamppInspection, XamppPreview, XamppReadiness, XamppRecoveryPreview } from "../../types/codex";

export function XamppPage() {
  const { t } = useTranslation();
  const [ready, setReady] = useState<XamppReadiness | null>(null); const [preview, setPreview] = useState<XamppPreview | null>(null); const [inspection, setInspection] = useState<XamppInspection | null>(null); const [recovery, setRecovery] = useState<XamppRecoveryPreview | null>(null); const [confirmation, setConfirmation] = useState(""); const [busy, setBusy] = useState(false); const [result, setResult] = useState(""); const [error, setError] = useState("");
  const run = async (action: () => Promise<void>) => { setBusy(true); setError(""); try { await action(); } catch { setError(t("xampp.error")); } finally { setBusy(false); } };
  return <>
    <PageHeader title={t("xampp.title")} description={t("xampp.description")} />
    <XamppDomains />
    <section className="notice-card"><h2>{t("xampp.boundaryTitle")}</h2><p>{t("xampp.boundary")}</p></section>
    <section className="backup-preview"><h2>{t("xampp.readiness")}</h2><p>{ready ? (ready.installationFound && ready.stopped ? t("xampp.ready") : t("xampp.notReady")) : t("xampp.readinessDescription")}</p><button disabled={busy} onClick={() => void run(async () => setReady(await getXamppReadiness()))}>{t("xampp.check")}</button></section>
    <section className="backup-preview"><h2>{t("xampp.export")}</h2><p>{t("xampp.exportDescription")}</p><button disabled={busy || !ready?.installationFound} onClick={() => void run(async () => { const output = await previewXampp(); if (output) setPreview(output); })}>{t("xampp.select")}</button>
      {preview && <div><p>{preview.sourceAppVersion} · {preview.architecture} · {preview.projects.join(", ")} · {preview.fileCount} {t("xampp.files")}</p><p>{t("xampp.config")}: {preview.configFiles.join(", ")}</p><p>{t("xampp.excluded", { count: preview.excludedCount })}</p><button disabled={busy || !ready?.installationFound} onClick={() => void run(async () => { setResult((await createXamppBundle(preview.token)).bundleName); setPreview(null); })}>{t("xampp.create")}</button></div>}
    </section>
    <section className="backup-preview"><h2>{t("xampp.recover")}</h2><p>{t("xampp.recoverDescription")}</p><button disabled={busy || !ready?.installationFound} onClick={() => void run(async () => { setInspection(await inspectXamppBundle()); setRecovery(null); setConfirmation(""); })}>{t("xampp.inspect")}</button>
      {inspection && <div><p>{inspection.bundleName} · {inspection.sourceAppVersion} · {inspection.architecture} · {inspection.fileCount} {t("xampp.files")} · SHA-256 ✓</p><button disabled={busy || !ready?.installationFound} onClick={() => void run(async () => { setRecovery(await previewXamppRecovery(inspection.token)); setConfirmation(""); })}>{t("xampp.previewRecovery")}</button></div>}
      {recovery && <div><p>{t("xampp.conflicts", { count: recovery.destinationConflicts })}</p><p><code>{recovery.stagingPath}</code></p><label className="delete-confirmation">{t("xampp.confirm")}<input disabled={busy || !ready?.installationFound} value={confirmation} onChange={(event) => setConfirmation(event.target.value)} /></label><button disabled={busy || !ready?.installationFound || confirmation !== "RECOVER"} onClick={() => void run(async () => { const output = await recoverXampp(recovery.token, confirmation); setResult(output.stagingPath); setRecovery(null); setConfirmation(""); })}>{t("xampp.recover")}</button></div>}
    </section>
    {result && <section className="success-card"><h2>{t("xampp.completed")}</h2><p><code>{result}</code></p><p>{t("xampp.manual")}</p></section>}
    {error && <p className="error-banner">{error}</p>}
  </>;
}
