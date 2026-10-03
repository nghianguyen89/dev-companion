import { useCallback, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { useAsyncValue } from "../../hooks/useAsyncValue";
import { useTranslation } from "../../i18n";
import { getPets, installPet, removePet } from "../../services/tauri";

export function PetsPage() {
  const { t } = useTranslation();
  const overview = useAsyncValue(useCallback(() => getPets(), []));
  const [busy, setBusy] = useState(false); const [message, setMessage] = useState(""); const [error, setError] = useState("");
  const run = async (action: () => Promise<{ path: string } | null>, success: "pets.installed" | "pets.removed") => {
    setBusy(true); setError(""); setMessage("");
    try { const result = await action(); if (result) { setMessage(t(success, { path: result.path })); await overview.refresh(); } }
    catch (reason) { setError(reason instanceof Error ? reason.message : t("pets.error")); }
    finally { setBusy(false); }
  };
  const remove = (id: string) => { if (window.prompt(t("pets.confirmRemove", { id })) === "REMOVE") void run(() => removePet(id, "REMOVE"), "pets.removed"); };
  return <>
    <PageHeader title={t("pets.title")} description={t("pets.description")} action={<div className="codex-environment-actions"><button type="button" disabled={busy || overview.loading} onClick={() => void overview.refresh()}>{t("common.refresh")}</button><button type="button" disabled={busy} onClick={() => void run(installPet, "pets.installed")}>{t("pets.install")}</button></div>} />
    <p className="notice-card">{t("pets.boundary")}</p>
    {overview.error && <p role="alert" className="error-banner">{overview.error}</p>}{error && <p role="alert" className="error-banner">{error}</p>}
    {overview.value && <><section className="codex-environment-card"><h2>{t("pets.installedCount", { count: overview.value.pets.length })}</h2><p><code>{overview.value.directory}</code></p>{overview.value.skipped > 0 && <p>{t("pets.skipped", { count: overview.value.skipped })}</p>}</section>
      {overview.value.pets.length === 0 ? <section className="notice-card"><h2>{t("pets.noPets")}</h2><p>{t("pets.restart")}</p></section> : <section className="codex-environment-list" aria-busy={busy}>{overview.value.pets.map((pet) => <article key={pet.id} className="codex-environment-card"><header><div><h2>{pet.displayName}</h2><p><code>{pet.id}</code></p></div></header><p>{pet.description}</p><dl><div><dt>{t("pets.version")}</dt><dd>v{pet.spriteVersionNumber}</dd></div><div><dt>{t("pets.sprite")}</dt><dd><code>{pet.spriteFile}</code></dd></div></dl><div className="codex-environment-actions"><button type="button" className="danger-button" disabled={busy} onClick={() => remove(pet.id)}>{t("pets.remove")}</button></div></article>)}</section>}</>}
    {message && <section role="status" className="success-card"><h2>{t("pets.completed")}</h2><p>{message}</p><p>{t("pets.restart")}</p></section>}
  </>;
}
