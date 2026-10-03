import { useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import { useAsyncValue } from "../../hooks/useAsyncValue";
import { useTranslation } from "../../i18n";
import { addCodexLauncherDirToUserPath, deleteCodexEnvironment, getCodexEnvironmentInstructions, getCodexEnvironmentOverview, openCodexEnvironmentAgents, openCodexEnvironmentHome, regenerateCodexLauncher, removeCodexLauncher, runCodexEnvironmentAction, saveCodexEnvironment, updateCodexEnvironmentInstructions } from "../../services/tauri";
import type { CodexEnvironmentInput, CodexEnvironmentStatus } from "../../types/codex";

const emptyForm = (): CodexEnvironmentInput => ({ displayName: "", commandAlias: "", codexHome: "%USERPROFILE%\\.codex-" });

export function CodexEnvironmentsPage() {
  const { t } = useTranslation();
  const overview = useAsyncValue(getCodexEnvironmentOverview);
  const [form, setForm] = useState<CodexEnvironmentInput | null>(null);
  const [instructions, setInstructions] = useState<{ id: string; content: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const cliInstalled = overview.value?.cli.installed ?? false;
  const run = async (action: () => Promise<{ message?: string } | void>, refresh = true) => {
    setBusy(true); setError(""); setMessage("");
    try { const result = await action(); setMessage(result?.message ?? ""); if (refresh) await overview.refresh(); }
    catch (reason) { setError(typeof reason === "string" ? reason : t("codexAccounts.actionFailed")); }
    finally { setBusy(false); }
  };
  const edit = (environment: CodexEnvironmentStatus) => setForm({ id: environment.id, displayName: environment.displayName, commandAlias: environment.commandAlias, codexHome: environment.codexHome, description: environment.description ?? "" });
  const editInstructions = (environment: CodexEnvironmentStatus) => void run(async () => { const value = await getCodexEnvironmentInstructions(environment.id); setInstructions({ id: environment.id, content: value.content }); }, false);
  const remove = (environment: CodexEnvironmentStatus) => { if (window.prompt(t("codexAccounts.confirmDelete")) === "DELETE") void run(() => deleteCodexEnvironment(environment.id, "DELETE")); };
  const removeScript = (environment: CodexEnvironmentStatus) => { if (window.prompt(t("codexAccounts.confirmRemoveLauncher")) === "REMOVE") void run(() => removeCodexLauncher(environment.id, "REMOVE")); };
  return <>
    <PageHeader title={t("codexAccounts.title")} description={t("codexAccounts.description")} action={<button type="button" onClick={() => void run(async () => undefined)} disabled={busy}>{t("codexAccounts.check")}</button>} />
    <section className="codex-environment-card">
      <h2>{t("codexAccounts.cli")}</h2>
      <dl><div><dt>{t("codexAccounts.cli")}</dt><dd>{overview.value?.cli.installed ? t("codexAccounts.installed") : t("codexAccounts.notInstalled")}</dd></div><div><dt>{t("codexAccounts.version")}</dt><dd><code>{overview.value?.cli.version ?? "—"}</code></dd></div><div><dt>{t("codexAccounts.path")}</dt><dd><code>{overview.value?.cli.path ?? "—"}</code></dd></div></dl>
      <p><strong>{t("codexAccounts.launcher")}:</strong> <code>{overview.value?.launcherDir ?? "—"}</code> · {overview.value?.launcherDirInUserPath ? t("codexAccounts.installed") : <button disabled={busy} type="button" onClick={() => void run(addCodexLauncherDirToUserPath)}>{t("codexAccounts.addPath")}</button>}</p>
      <p>{t("codexAccounts.restart")}</p>
    </section>
    <p className="notice-card">{t("codexAccounts.defaultBoundary")}</p>
    <section className="codex-environment-heading"><h2>{t("codexAccounts.title")}</h2><button type="button" disabled={busy} onClick={() => { setForm(emptyForm()); setInstructions(null); }}>{t("codexAccounts.add")}</button></section>
    {form && <section className="codex-environment-card"><h2>{form.id ? t("codexAccounts.edit") : t("codexAccounts.add")}</h2><form className="codex-environment-form" onSubmit={(event) => { event.preventDefault(); void run(async () => { await saveCodexEnvironment(form); setForm(null); }); }}>
      <label>{t("codexAccounts.name")}<input required maxLength={80} value={form.displayName} onChange={(event) => setForm({ ...form, displayName: event.target.value })} /></label>
      <label>{t("codexAccounts.alias")}<input required maxLength={64} value={form.commandAlias} onChange={(event) => setForm({ ...form, commandAlias: event.target.value })} /></label>
      <label>{t("codexAccounts.home")}<input required value={form.codexHome} onChange={(event) => setForm({ ...form, codexHome: event.target.value })} /></label>
      <label>{t("codexAccounts.descriptionLabel")}<input maxLength={500} value={form.description ?? ""} onChange={(event) => setForm({ ...form, description: event.target.value })} /></label>
      <div><button disabled={busy} type="submit">{t("codexAccounts.save")}</button><button disabled={busy} type="button" onClick={() => setForm(null)}>{t("codexAccounts.cancel")}</button></div>
    </form></section>}
    {overview.error && <p role="alert" className="error-banner">{overview.error}</p>}
    <section className="codex-environment-list" aria-busy={overview.loading || busy}>{overview.value?.environments.map((environment) => <article key={environment.id} className="codex-environment-card">
      <header><div><h2>{environment.displayName} {environment.isDefault && <small>{t("codexAccounts.default")}</small>}</h2><p><code>{environment.commandAlias}</code> · <code>{environment.codexHome}</code></p></div><strong className={`login-${environment.loginStatus}`}>{t(`codexAccounts.${environment.loginStatus}`)}</strong></header>
      {environment.description && <p>{environment.description}</p>}
      <dl><div><dt>{t("codexAccounts.agents")}</dt><dd>{environment.agentsExists ? t("codexAccounts.exists") : t("codexAccounts.missing")} · {environment.managesInstructions ? t("codexAccounts.managed") : t("codexAccounts.custom")}</dd></div><div><dt>{t("codexAccounts.launcher")}</dt><dd><code>{environment.launcherPath ?? t("codexAccounts.noLauncher")}</code></dd></div></dl>
      <div className="codex-environment-actions"><button disabled={busy} onClick={() => void run(() => openCodexEnvironmentHome(environment.id), false)}>{t("codexAccounts.open")}</button><button disabled={busy || !cliInstalled} onClick={() => void run(() => runCodexEnvironmentAction(environment.id, "launch"), false)}>{t("codexAccounts.launch")}</button><button disabled={busy || !cliInstalled} onClick={() => void run(() => runCodexEnvironmentAction(environment.id, "login"), false)}>{t("codexAccounts.login")}</button><button disabled={busy || !cliInstalled} onClick={() => void run(() => runCodexEnvironmentAction(environment.id, "logout"), false)}>{t("codexAccounts.logout")}</button><button disabled={busy} onClick={() => void navigator.clipboard.writeText(environment.commandAlias)}>{t("codexAccounts.copy")}</button>
        {!environment.isDefault && <><button disabled={busy} onClick={() => edit(environment)}>{t("codexAccounts.edit")}</button><button disabled={busy} onClick={() => void run(() => regenerateCodexLauncher(environment.id))}>{t("codexAccounts.launcherAction")}</button><button disabled={busy || !environment.launcherPath} onClick={() => removeScript(environment)}>{t("codexAccounts.removeLauncher")}</button><button disabled={busy} onClick={() => editInstructions(environment)}>{t("codexAccounts.instructions")}</button><button disabled={busy || !environment.agentsExists} onClick={() => void run(() => openCodexEnvironmentAgents(environment.id), false)}>{t("codexAccounts.editFile")}</button><button disabled={busy} className="danger-button" onClick={() => remove(environment)}>{t("codexAccounts.removeEntry")}</button></>}
      </div>
    </article>)}</section>
    {instructions && <section className="codex-environment-card"><h2>{t("codexAccounts.instructions")}</h2><p>{t("codexAccounts.managed")}</p><textarea value={instructions.content} maxLength={20_000} onChange={(event) => setInstructions({ ...instructions, content: event.target.value })} /><div className="codex-environment-actions"><button disabled={busy} onClick={() => void run(async () => { await updateCodexEnvironmentInstructions(instructions.id, instructions.content); setInstructions(null); })}>{t("codexAccounts.saveInstructions")}</button><button disabled={busy} onClick={() => setInstructions(null)}>{t("codexAccounts.cancel")}</button></div></section>}
    {error && <p role="alert" className="error-banner">{error}</p>}{message && <p role="status" className="notice-card">{message}</p>}
  </>;
}
