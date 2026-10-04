import { PageHeader } from "../../components/PageHeader";
import { appVersion, releaseNotes } from "../../app/releaseNotes";
import { useTranslation } from "../../i18n";

export function AboutPage() {
  const { language, t } = useTranslation();
  return <><PageHeader title={t("about.title")} description={language === "vi" ? "Phiên bản hiện tại và nhật ký thay đổi của Dev Companion." : "Current version and update history for Dev Companion."} />
    <section className="about-card"><h2>Dev Companion</h2><dl><div><dt>{language === "vi" ? "Phiên bản" : "Version"}</dt><dd><code>{appVersion}</code></dd></div><div><dt>{language === "vi" ? "Phạm vi" : "Scope"}</dt><dd>{language === "vi" ? "Công cụ phát triển cục bộ cho Windows" : "Windows-first local developer utility"}</dd></div></dl></section>
    <section className="about-card"><h2>{language === "vi" ? "Nhật ký cập nhật" : "Update history"}</h2><ol className="release-notes">{releaseNotes[language].map((release) => <li key={release.version}><h3>{release.version} · {release.title}</h3><ul>{release.changes.map((change) => <li key={change}>{change}</li>)}</ul></li>)}</ol></section>
  </>;
}
