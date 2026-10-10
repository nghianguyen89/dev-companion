import type { ReactNode } from "react";
import { useTranslation } from "../../i18n";
import type { XamppDomain } from "../../types/codex";
import { xamppDomainUrl } from "../../services/tauri";

type Icon = "ssl" | "www" | "open" | "copy" | "check" | "folder" | "listing" | "edit" | "delete";
function DomainIcon({ icon }: { icon: Icon }) {
  const paths: Record<Exclude<Icon, "www">, string> = {
    ssl: "M12 3 4 6v6c0 5 8 9 8 9s8-4 8-9V6l-8-3ZM9 12v-2a3 3 0 0 1 6 0v2m-6 0h6v5H9z",
    open: "M14 3h7v7m0-7L10 14M10 5H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-5",
    copy: "M9 9h12v12H9zM15 9V3H3v12h6",
    check: "m5 12 4 4L19 6",
    folder: "M3 7V4h6l3 3h9v3M3 7h18l-3 13H3z",
    listing: "M8 4H5v17h14V4h-3M8 3h8v4H8zM8 11l1 1 2-2m2 1h3M8 16l1 1 2-2m2 1h3",
    edit: "m14 5 5 5M4 20l5-1L21 7l-5-5L4 14v6z",
    delete: "M3 6h18M8 6V3h8v3M5 6l1 15h12l1-15M10 10v7m4-7v7",
  };
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {icon === "www" ? <><rect x="1" y="5" width="22" height="14" rx="3" /><text x="12" y="14.5" textAnchor="middle" fill="currentColor" stroke="none" fontSize="7" fontWeight="700">WWW</text></> : <path d={paths[icon]} />}
  </svg>;
}

interface Props {
  domain: XamppDomain;
  httpsReady: boolean;
  busy: boolean;
  actionsAvailable: boolean;
  copied?: boolean;
  onOpen: () => void;
  onCopy: () => void;
  onOpenFolder: () => void;
  onToggleListing: () => void;
  onEdit: () => void;
  onDelete: () => void;
  children?: ReactNode;
}

export function XamppDomainEntry({ domain, httpsReady, busy, actionsAvailable, copied = false, onOpen, onCopy, onOpenFolder, onToggleListing, onEdit, onDelete, children }: Props) {
  const { t } = useTranslation();
  const httpsEnabled = httpsReady && domain.redirectHttps;
  const url = xamppDomainUrl(domain.name, httpsEnabled);
  const sslLabel = t(httpsEnabled ? "xamppDomains.sslEnabled" : "xamppDomains.sslDisabled");
  const wwwLabel = t(domain.www ? "xamppDomains.wwwEnabled" : "xamppDomains.wwwDisabled");
  const listingLabel = t(domain.directoryListing ? "xamppDomains.disableListing" : "xamppDomains.enableListing");
  return <div className="xampp-domain-entry">
    <div className="xampp-domain-row">
      <div className="xampp-domain-details">
        <div className="xampp-domain-heading">
          <span className={`xampp-domain-status ssl${httpsEnabled ? " active" : ""}`} title={sslLabel} aria-label={sslLabel} role="img"><DomainIcon icon="ssl" /></span>
          <span className={`xampp-domain-status www${domain.www ? " active" : ""}`} title={wwwLabel} aria-label={wwwLabel} role="img"><DomainIcon icon="www" /></span>
          <strong className="xampp-domain-name">{domain.name}</strong>
          <div className="xampp-domain-links">
            <button type="button" className="xampp-domain-icon-button" disabled={busy} title={`${t("xamppDomains.openDomain")}: ${url}`} aria-label={t("xamppDomains.openDomain")} onClick={onOpen}><DomainIcon icon="open" /></button>
            <button type="button" className={`xampp-domain-icon-button${copied ? " copied" : ""}`} disabled={busy} title={`${t(copied ? "xamppDomains.domainCopied" : "xamppDomains.copyDomain")}: ${url}`} aria-label={t("xamppDomains.copyDomain")} onClick={onCopy}><DomainIcon icon={copied ? "check" : "copy"} /><span className="xampp-domain-copy-feedback" role="status">{copied ? t("xamppDomains.domainCopied") : ""}</span></button>
          </div>
        </div>
        <button type="button" className="xampp-domain-folder" disabled={busy} title={`${t("xamppDomains.openFolder")}: ${domain.folder}`} aria-label={`${t("xamppDomains.openFolder")}: ${domain.folder}`} onClick={onOpenFolder}><DomainIcon icon="folder" /><code>{domain.folder}</code></button>
        {domain.lan && <span className="xampp-domain-lan">{t("xamppDomains.lan")}</span>}
      </div>
      <div className="xampp-domain-actions">
        <span title={listingLabel}><button type="button" className={`xampp-domain-icon-button${domain.directoryListing ? " listing-active" : ""}`} disabled={busy || !actionsAvailable} title={listingLabel} aria-label={listingLabel} aria-pressed={domain.directoryListing} onClick={onToggleListing}><DomainIcon icon="listing" /></button></span>
        <span title={t("xamppDomains.edit")}><button type="button" className="xampp-domain-icon-button" disabled={busy || !actionsAvailable} title={t("xamppDomains.edit")} aria-label={t("xamppDomains.edit")} onClick={onEdit}><DomainIcon icon="edit" /></button></span>
        <span title={t("xamppDomains.delete")}><button type="button" className="xampp-domain-icon-button delete" disabled={busy || !actionsAvailable || domain.name === "localhost"} title={t("xamppDomains.delete")} aria-label={t("xamppDomains.delete")} onClick={onDelete}><DomainIcon icon="delete" /></button></span>
      </div>
    </div>
    {children}
  </div>;
}
