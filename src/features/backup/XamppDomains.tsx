import { useEffect, useRef, useState } from "react";
import { useTranslation } from "../../i18n";
import { deleteXamppDomain, elevateXamppManager, exportXamppCa, getXamppDomains, initializeXamppDomains, openXamppDomain, openXamppDomainFolder, pickCompressionFolder, pruneXamppBackups, saveXamppDomain, setXamppBackupRetention, setXamppInstallation, xamppDomainUrl } from "../../services/tauri";
import { XamppDomainEntry } from "./XamppDomainEntry";
import type { XamppCaInput, XamppDomain, XamppDomainAction, XamppDomainsOverview } from "../../types/codex";

const emptyDomain: XamppDomain = { name: "", folder: "", www: false, redirectHttps: true, directoryListing: false, lan: false };
const emptyCa: XamppCaInput = { commonName: "", organization: "", unit: "", country: "", state: "", city: "", email: "", days: 3650 };

export function XamppDomains() {
  const { t } = useTranslation();
  const [overview, setOverview] = useState<XamppDomainsOverview | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState("");
  const [result, setResult] = useState<XamppDomainAction | null>(null);
  const [ca, setCa] = useState(emptyCa);
  const [domain, setDomain] = useState(emptyDomain);
  const [previousName, setPreviousName] = useState<string | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const [copiedName, setCopiedName] = useState<string | null>(null);
  const [adminNotice, setAdminNotice] = useState<string | null>(null);
  const [keepRecent, setKeepRecent] = useState(10);
  const domainForm = useRef<HTMLFormElement>(null);
  useEffect(() => {
    if (!copiedName) return;
    const timer = window.setTimeout(() => setCopiedName(null), 2000);
    return () => window.clearTimeout(timer);
  }, [copiedName]);
  useEffect(() => {
    let active = true;
    getXamppDomains().then((value) => { if (active) setOverview(value); }).catch((reason: unknown) => { if (active) setError(reason instanceof Error ? reason.message : String(reason)); }).finally(() => { if (active) setBusy(false); });
    return () => { active = false; };
  }, []);
  useEffect(() => { if (overview) setKeepRecent(overview.backups.keepRecent); }, [overview]);
  const run = async (action: () => Promise<void>, refresh = true) => {
    setBusy(true); setError(""); setResult(null);
    try { await action(); } catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
    if (refresh) {
      try { setOverview(await getXamppDomains()); } catch (reason) { setOverview(null); setError((current) => [current, reason instanceof Error ? reason.message : String(reason)].filter(Boolean).join("\n")); }
    }
    setBusy(false);
  };
  const canManage = Boolean(overview?.installationPath && overview.administrator && overview.initialized && overview.caReady && overview.caTrusted);
  const actionsAvailable = Boolean(overview?.installationPath && overview.initialized);
  const httpsReady = Boolean(overview?.initialized && overview.caReady);
  const resetDomain = () => { setDomain(emptyDomain); setPreviousName(null); };
  const copyDomain = async (item: XamppDomain) => {
    try { await navigator.clipboard.writeText(xamppDomainUrl(item.name, httpsReady && item.redirectHttps)); setCopiedName(item.name); }
    catch (reason) { setError(reason instanceof Error ? reason.message : String(reason)); }
  };
  return <section className="file-transfer-card xampp-domains" aria-busy={busy}>
    <h2>{t("xamppDomains.title")}</h2><p>{t("xamppDomains.description")}</p>
    <div className="file-transfer-actions">
      <button disabled={busy} onClick={() => void run(async () => setOverview(await getXamppDomains()), false)}>{t("xamppDomains.refresh")}</button>
      <button disabled={busy} onClick={() => void run(async () => { const path = await pickCompressionFolder(t("xamppDomains.selectInstallation")); if (path) setOverview(await setXamppInstallation(path)); }, false)}>{t("xamppDomains.selectInstallation")}</button>
      {overview && !overview.administrator && <button disabled={busy} onClick={() => void run(async () => setResult(await elevateXamppManager()), false)}>{t("xamppDomains.elevate")}</button>}
    </div>
    {busy && <p role="status">{t("xamppDomains.pending")}</p>}
    {overview && <>
      <p>{t("xamppDomains.installation")}: <code>{overview.installationPath ?? t("xamppDomains.notFound")}</code></p>
      <p>{t("xamppDomains.config")}: <code>{overview.configDirectory}</code></p>
      <p>{overview.message}</p>
      {!overview.administrator && <p>{t("xamppDomains.adminRequired")}</p>}
      <p>{t(overview.caTrusted ? "xamppDomains.trusted" : "xamppDomains.notTrusted")}</p>
      <fieldset disabled={busy || !overview.installationPath || !overview.administrator}>
        <legend>{t("xamppRetention.title")}</legend>
        <p>{t("xamppRetention.summary", { count: overview.backups.count, bytes: overview.backups.totalBytes.toLocaleString() })}</p>
        <p>{t("xamppRetention.policy", { count: overview.backups.keepRecent })}</p>
        <label>{t("xamppRetention.keepRecent")}<input type="number" required min="1" max="100" value={keepRecent} onChange={(event) => setKeepRecent(Number(event.target.value))} /></label>
        <div className="file-transfer-actions">
          <button type="button" disabled={keepRecent < 1 || keepRecent > 100 || !Number.isInteger(keepRecent)} onClick={() => void run(async () => setOverview(await setXamppBackupRetention(keepRecent)), false)}>{t("xamppRetention.save")}</button>
          <button type="button" disabled={overview.backups.prunableCount === 0} onClick={() => void run(async () => setResult(await pruneXamppBackups()))}>{t("xamppRetention.clean")}</button>
        </div>
      </fieldset>
      {(!overview.initialized || !overview.caReady || !overview.caTrusted) && <form onSubmit={(event) => { event.preventDefault(); void run(async () => setResult(await initializeXamppDomains(ca))); }}>
        <fieldset disabled={busy || !overview.installationPath || !overview.administrator}>
          <legend>{t("xamppDomains.setup")}</legend>
          <p>{t("xamppDomains.setupDescription")}</p>
          {overview.caReady ? <p>{t("xamppDomains.reuseCa")}</p> : <div className="file-transfer-advanced">
            {(["commonName", "organization", "unit", "country", "state", "city", "email"] as const).map((key) => <label key={key}>{t(`xamppDomains.${key}`)}<input type={key === "email" ? "email" : "text"} required={key === "commonName" || key === "country"} minLength={key === "country" ? 2 : undefined} maxLength={key === "country" ? 2 : undefined} value={ca[key]} onChange={(event) => setCa({ ...ca, [key]: event.target.value })} /></label>)}
            <label>{t("xamppDomains.days")}<input type="number" required min="30" max="3650" value={ca.days} onChange={(event) => setCa({ ...ca, days: Number(event.target.value) })} /></label>
          </div>}
          <div className="file-transfer-actions"><button type="submit">{t("xamppDomains.initialize")}</button></div>
        </fieldset>
      </form>}
    </>}
    <form ref={domainForm} onSubmit={(event) => { event.preventDefault(); if (!canManage) return; void run(async () => { setResult(await saveXamppDomain(domain, previousName)); resetDomain(); }); }}>
      <fieldset disabled={busy || !actionsAvailable}>
        <legend>{t(previousName ? "xamppDomains.editDomain" : "xamppDomains.addDomain")}</legend>
        <div className="file-transfer-advanced">
          <label>{t("xamppDomains.name")}<input required placeholder="project.test" value={domain.name} onChange={(event) => setDomain({ ...domain, name: event.target.value })} /></label>
          <label>{t("xamppDomains.folder")}<input required value={domain.folder} onChange={(event) => setDomain({ ...domain, folder: event.target.value })} /><button type="button" onClick={() => void run(async () => { const folder = await pickCompressionFolder(t("xamppDomains.folder")); if (folder) setDomain({ ...domain, folder }); }, false)}>{t("xamppDomains.browse")}</button></label>
          {(["www", "redirectHttps", "directoryListing", "lan"] as const).map((key) => <label className="checkbox-row" key={key}><input type="checkbox" checked={domain[key]} onChange={(event) => setDomain({ ...domain, [key]: event.target.checked })} />{t(`xamppDomains.${key}`)}</label>)}
        </div>
        {!overview?.administrator && actionsAvailable && <p>{t("xamppDomains.adminRequired")}</p>}
        <div className="file-transfer-actions"><button type="submit" disabled={!canManage}>{t("xamppDomains.save")}</button>{previousName && <button type="button" onClick={resetDomain}>{t("xamppDomains.cancel")}</button>}{overview && !overview.administrator && <button type="button" onClick={() => void run(async () => setResult(await elevateXamppManager()), false)}>{t("xamppDomains.elevate")}</button>}</div>
      </fieldset>
    </form>
    <h3>{t("xamppDomains.domains")}</h3>
    {overview?.domains.length === 0 && <p>{t("xamppDomains.empty")}</p>}
    {overview?.domains.map((item) => <XamppDomainEntry key={item.name} domain={item} httpsReady={httpsReady} busy={busy} actionsAvailable={actionsAvailable} copied={copiedName === item.name}
      onOpen={() => void run(async () => { await openXamppDomain(item.name, httpsReady && item.redirectHttps); }, false)}
      onCopy={() => void copyDomain(item)}
      onOpenFolder={() => void run(async () => { await openXamppDomainFolder(item.name); }, false)}
      onEdit={() => { setDomain({ ...item }); setPreviousName(item.name); setDeleting(null); setConfirmation(""); setAdminNotice(!overview.administrator ? item.name : null); requestAnimationFrame(() => { domainForm.current?.scrollIntoView({ behavior: "smooth", block: "start" }); domainForm.current?.querySelector("input")?.focus({ preventScroll: true }); }); }}
      onToggleListing={() => { if (!overview.administrator) { setAdminNotice(item.name); return; } void run(async () => { setResult(await saveXamppDomain({ ...item, directoryListing: !item.directoryListing }, item.name)); if (previousName === item.name) setDomain((current) => ({ ...current, directoryListing: !item.directoryListing })); }); }}
      onDelete={() => { setDeleting(item.name); setConfirmation(""); setAdminNotice(!overview.administrator ? item.name : null); }}>
      {adminNotice === item.name && !overview.administrator && <div className="xampp-domain-admin-notice" role="status"><p>{t("xamppDomains.adminRequired")}</p><button type="button" disabled={busy} onClick={() => void run(async () => setResult(await elevateXamppManager()), false)}>{t("xamppDomains.elevate")}</button></div>}
      {deleting === item.name && <fieldset disabled={busy}>
        <label className="delete-confirmation">{t("xamppDomains.deleteConfirmation", { name: item.name })}<input value={confirmation} onChange={(event) => setConfirmation(event.target.value)} /></label>
        <div className="file-transfer-actions"><button disabled={!canManage || confirmation !== item.name} onClick={() => void run(async () => { setResult(await deleteXamppDomain(item.name, confirmation)); setDeleting(null); setConfirmation(""); if (previousName === item.name) resetDomain(); })}>{t("xamppDomains.delete")}</button><button onClick={() => { setDeleting(null); setConfirmation(""); setAdminNotice(null); }}>{t("xamppDomains.cancel")}</button></div>
      </fieldset>}
    </XamppDomainEntry>)}
    <h3>{t("xamppDomains.lanTitle")}</h3><p>{t("xamppDomains.lanHelp")}</p>
    {overview && <p>{t("xamppDomains.lanAddresses")}: <code>{overview.lanAddresses.join(", ") || t("xamppDomains.noLan")}</code></p>}
    <div className="file-transfer-actions"><button disabled={busy || !overview?.caReady} onClick={() => void run(async () => { const destination = await pickCompressionFolder(t("xamppDomains.exportCa")); if (destination) setResult(await exportXamppCa(destination)); })}>{t("xamppDomains.exportCa")}</button></div>
    {result && <div className="success-card" role="status"><p>{result.message}</p>{result.safetyPath && <p>{t("xamppDomains.safety")}: <code>{result.safetyPath}</code></p>}</div>}
    {error && <p className="error-banner" role="alert">{error}</p>}
  </section>;
}
