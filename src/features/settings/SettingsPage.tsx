import { useEffect, useState } from "react";
import { PageHeader } from "../../components/PageHeader";
import type { AppConfiguration } from "../../types/codex";
import { useTranslation } from "../../i18n";

interface SettingsPageProps { configuration: AppConfiguration | null; loadError?: string | null; onSave: (configuration: AppConfiguration) => Promise<void>; }

export function SettingsPage({ configuration, loadError, onSave }: SettingsPageProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<AppConfiguration | null>(configuration);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => setDraft(configuration), [configuration]);
  if (!draft) return <><PageHeader title={t("settings.title")} description={t("settings.description")} />{loadError ? <p role="alert" className="error-banner">{loadError}</p> : <p>{t("common.loading")}</p>}</>;
  const save = async () => { setSaving(true); setError(null); try { await onSave(draft); } catch (reason) { setError(typeof reason === "string" ? reason : t("common.errorOperation")); } finally { setSaving(false); } };
  return <>
    <PageHeader title={t("settings.title")} description={t("settings.description")} />
    <section className="settings-form">
      <label>{t("settings.language")}<select value={draft.language} onChange={(event) => setDraft({ ...draft, language: event.target.value as AppConfiguration["language"] })}><option value="en">{t("settings.english")}</option><option value="vi">{t("settings.vietnamese")}</option></select></label>
      <label>{t("settings.theme")}<select value={draft.theme} onChange={(event) => setDraft({ ...draft, theme: event.target.value as AppConfiguration["theme"] })}><option value="system">{t("settings.system")}</option><option value="light">{t("settings.light")}</option><option value="dark">{t("settings.dark")}</option></select></label>
      <label>{t("settings.logLevel")}<select value={draft.logLevel} onChange={(event) => setDraft({ ...draft, logLevel: event.target.value as AppConfiguration["logLevel"] })}><option value="error">{t("settings.error")}</option><option value="warn">{t("settings.warn")}</option><option value="info">{t("settings.info")}</option><option value="debug">{t("settings.debug")}</option></select></label>
      <label className="checkbox-row"><input type="checkbox" checked={draft.portableMode} onChange={(event) => setDraft({ ...draft, portableMode: event.target.checked })} /> {t("settings.portable")}</label>
      <label className="checkbox-row"><input type="checkbox" checked={draft.createSafetyBackups} onChange={(event) => setDraft({ ...draft, createSafetyBackups: event.target.checked })} /> {t("settings.safety")}</label>
      <button type="button" onClick={() => void save()} disabled={saving}>{saving ? t("common.working") : t("common.save")}</button>
      {error && <p role="alert" className="error-banner">{error}</p>}
    </section>
  </>;
}
