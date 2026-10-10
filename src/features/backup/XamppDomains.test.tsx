import { afterEach, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { XamppDomains } from "./XamppDomains";
import { XamppPage } from "./XamppPage";
import { XamppDomainEntry } from "./XamppDomainEntry";
import { deleteXamppDomain, elevateXamppManager, exportXamppCa, getXamppDomains, initializeXamppDomains, openXamppDomain, openXamppDomainFolder, pruneXamppBackups, saveXamppDomain, setXamppBackupRetention, setXamppInstallation, xamppDomainUrl } from "../../services/tauri";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(), Channel: vi.fn() }));
import { invoke } from "@tauri-apps/api/core";
const nativeInvoke = vi.mocked(invoke);
afterEach(() => { nativeInvoke.mockReset(); vi.unstubAllGlobals(); });

it("keeps domain edits locked until native setup status is available", () => {
  const html = renderToStaticMarkup(<XamppDomains />);
  expect(html).toContain('aria-busy="true"');
  expect(html).toContain('<fieldset disabled="">');
  expect(html).toContain("Add domain");
  expect(html).toContain("never share the CA private key");
  expect(html).toContain("Export public CA certificate");
});

it("places domain management before the preserved backup workflow", () => {
  const html = renderToStaticMarkup(<XamppPage />);
  expect(html.indexOf("XAMPP domains &amp; HTTPS")).toBeLessThan(html.indexOf("XAMPP file bundle"));
  expect(html).toContain("Create XAMPP file bundle");
  expect(html).toContain("Inspect XAMPP bundle");
  expect(html).toContain('<button disabled="">Select htdocs projects</button>');
});

it("forwards the domain manager contract without discarding options", async () => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  await getXamppDomains();
  expect(nativeInvoke).toHaveBeenLastCalledWith("get_xampp_domains", undefined);
  await openXamppDomain("example.test", true);
  expect(nativeInvoke).toHaveBeenLastCalledWith("open_xampp_domain", { name: "example.test", https: true });
  await openXamppDomainFolder("example.test");
  expect(nativeInvoke).toHaveBeenLastCalledWith("open_xampp_domain_folder", { name: "example.test" });
  await setXamppInstallation("C:\\xampp");
  expect(nativeInvoke).toHaveBeenLastCalledWith("set_xampp_installation", { path: "C:\\xampp" });
  await setXamppBackupRetention(10);
  expect(nativeInvoke).toHaveBeenLastCalledWith("set_xampp_backup_retention", { keepRecent: 10 });
  await pruneXamppBackups();
  expect(nativeInvoke).toHaveBeenLastCalledWith("prune_xampp_backups", undefined);
  const ca = { commonName: "Development CA", organization: "Example", unit: "Web", country: "VN", state: "", city: "", email: "", days: 3650 };
  await initializeXamppDomains(ca);
  expect(nativeInvoke).toHaveBeenLastCalledWith("initialize_xampp_domains", { ca });
  const domain = { name: "example.test", folder: "D:\\web", www: true, redirectHttps: true, directoryListing: false, lan: true };
  await saveXamppDomain(domain, "old.test");
  expect(nativeInvoke).toHaveBeenLastCalledWith("save_xampp_domain", { domain, previousName: "old.test" });
  await saveXamppDomain(domain, null);
  expect(nativeInvoke).toHaveBeenLastCalledWith("save_xampp_domain", { domain, previousName: null });
  await deleteXamppDomain(domain.name, domain.name);
  expect(nativeInvoke).toHaveBeenLastCalledWith("delete_xampp_domain", { name: domain.name, confirmation: domain.name });
  await exportXamppCa("D:\\certificates");
  expect(nativeInvoke).toHaveBeenLastCalledWith("export_xampp_ca", { destination: "D:\\certificates" });
  await elevateXamppManager();
  expect(nativeInvoke).toHaveBeenLastCalledWith("elevate_xampp_manager", undefined);
});

it("preserves native errors for display", async () => {
  vi.stubGlobal("window", { __TAURI_INTERNALS__: {} });
  nativeInvoke.mockRejectedValueOnce("Apache configuration test failed: line 42");
  await expect(getXamppDomains()).rejects.toBe("Apache configuration test failed: line 42");
});

it("shows enabled HTTPS per domain and uses the primary URL for open/copy", () => {
  const noop = () => {};
  const domain = { name: "example.test", folder: "D:\\web", www: true, redirectHttps: true, directoryListing: true, lan: false };
  const html = renderToStaticMarkup(<XamppDomainEntry domain={domain} httpsReady busy={false} actionsAvailable onOpen={noop} onCopy={noop} onOpenFolder={noop} onToggleListing={noop} onEdit={noop} onDelete={noop} />);
  expect(html).toContain('class="xampp-domain-status ssl active"');
  expect(html).toContain('class="xampp-domain-status www active"');
  expect(html).toContain('title="Open domain: https://example.test/"');
  expect(html).toContain('title="Copy domain URL: https://example.test/"');
  expect(html).not.toContain("https://www.example.test");
  expect(html).toContain('title="Open project folder: D:\\web"');
  expect(html).toContain('aria-pressed="true"');
  expect(html).toContain('title="Disable directory listing"');
});

it("mutes SSL and uses HTTP after the domain HTTPS redirect is unchecked even when CA is ready", () => {
  const noop = () => {};
  const domain = { name: "example.test", folder: "D:\\web", www: true, redirectHttps: false, directoryListing: false, lan: false };
  const html = renderToStaticMarkup(<XamppDomainEntry domain={domain} httpsReady busy={false} actionsAvailable onOpen={noop} onCopy={noop} onOpenFolder={noop} onToggleListing={noop} onEdit={noop} onDelete={noop} />);
  expect(html).toContain('class="xampp-domain-status ssl"');
  expect(html).toContain('class="xampp-domain-status www active"');
  expect(html).toContain('title="Open domain: http://example.test/"');
  expect(html).toContain('title="Copy domain URL: http://example.test/"');
});

it("keeps unavailable SSL/www muted and protects localhost delete while read-only actions remain usable", () => {
  const noop = () => {};
  const domain = { name: "localhost", folder: "C:\\xampp\\htdocs", www: false, redirectHttps: false, directoryListing: false, lan: false };
  const html = renderToStaticMarkup(<XamppDomainEntry domain={domain} httpsReady={false} busy={false} actionsAvailable onOpen={noop} onCopy={noop} onOpenFolder={noop} onToggleListing={noop} onEdit={noop} onDelete={noop} />);
  expect(html).toContain('class="xampp-domain-status ssl"');
  expect(html).toContain('class="xampp-domain-status www"');
  expect(html).toContain('title="Open domain: http://localhost/"');
  expect(html).toContain('disabled="" title="Delete"');
  expect(html).not.toContain('disabled="" title="Open domain');
  expect(xamppDomainUrl("example.test", true)).toBe("https://example.test/");
  expect(xamppDomainUrl("example.test", false)).toBe("http://example.test/");
});

it("shows copy feedback inside its existing button without a full-page operation status", () => {
  const noop = () => {};
  const domain = { name: "example.test", folder: "D:\\web", www: false, redirectHttps: false, directoryListing: false, lan: false };
  const html = renderToStaticMarkup(<XamppDomainEntry domain={domain} httpsReady busy={false} actionsAvailable copied onOpen={noop} onCopy={noop} onOpenFolder={noop} onToggleListing={noop} onEdit={noop} onDelete={noop} />);
  expect(html).toContain('class="xampp-domain-icon-button copied"');
  expect(html).toContain('class="xampp-domain-copy-feedback" role="status">Domain URL copied.');
  expect(html).not.toContain('disabled=""');
  expect(html).not.toContain("Working");
});
