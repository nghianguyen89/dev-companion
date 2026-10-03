import { useCallback, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { useAsyncValue } from "../../hooks/useAsyncValue";
import { useTranslation } from "../../i18n";
import { exportSkill, getSkills, importSkill } from "../../services/tauri";

export function SkillsPage() {
  const { t } = useTranslation();
  const overview = useAsyncValue(useCallback(() => getSkills(), []));
  const [busy, setBusy] = useState(false); const [message, setMessage] = useState(""); const [error, setError] = useState("");
  const run = async (action: () => Promise<{ path: string } | null>, success: "skills.imported" | "skills.exported") => {
    setBusy(true); setError(""); setMessage("");
    try { const result = await action(); if (result) { setMessage(t(success, { path: result.path })); await overview.refresh(); } }
    catch (reason) { setError(reason instanceof Error ? reason.message : t("skills.error")); }
    finally { setBusy(false); }
  };
  return <>
    <PageHeader title={t("skills.title")} description={t("skills.description")} action={<div className="codex-environment-actions"><button type="button" disabled={busy || overview.loading} onClick={() => void overview.refresh()}>{t("common.refresh")}</button><button type="button" disabled={busy} onClick={() => void run(importSkill, "skills.imported")}>{t("skills.import")}</button></div>} />
    <p className="notice-card">{t("skills.boundary")}</p>
    {overview.error && <p role="alert" className="error-banner">{overview.error}</p>}{error && <p role="alert" className="error-banner">{error}</p>}
    {overview.value && <><section className="codex-environment-card"><h2>{t("skills.installed", { count: overview.value.skills.length })}</h2><p><code>{overview.value.directory}</code></p>{overview.value.skipped > 0 && <p>{t("skills.skipped", { count: overview.value.skipped })}</p>}</section>
      {overview.value.skills.length === 0 ? <section className="notice-card"><h2>{t("skills.noSkills")}</h2><p>{t("skills.restart")}</p></section> : <section className="codex-environment-list" aria-busy={busy}>{overview.value.skills.map((skill) => <article key={skill.id} className="codex-environment-card"><header><div><h2>{skill.id}</h2><p><code>{t("skills.manifest", { bytes: skill.manifestBytes })}</code></p></div></header><div className="codex-environment-actions"><button type="button" disabled={busy} onClick={() => void run(() => exportSkill(skill.id), "skills.exported")}>{t("skills.export")}</button></div></article>)}</section>}</>}
    {message && <section role="status" className="success-card"><h2>{t("skills.completed")}</h2><p>{message}</p><p>{t("skills.restart")}</p></section>}
  </>;
}
