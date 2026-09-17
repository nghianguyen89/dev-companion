import { afterEach, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ConversationsPage } from "./ConversationsPage";
import { useAsyncValue } from "../../hooks/useAsyncValue";
import { I18nProvider, translate, type Language } from "../../i18n";

vi.mock("../../hooks/useAsyncValue", () => ({ useAsyncValue: vi.fn() }));
afterEach(() => vi.restoreAllMocks());

it.each<Language>(["en", "vi"])("reuses one date formatter for a large %s session table", (language) => {
  const timestamp = "2026-09-09T01:00:00Z";
  const expectedDate = new Intl.DateTimeFormat(language, { dateStyle: "medium", timeStyle: "short" }).format(new Date(timestamp));
  const conversations = Array.from({ length: 2000 }, (_, index) => ({
    id: `session-${index}`, title: `Session ${index}`, projectName: null,
    projectPath: null, source: "cli", createdAt: index === 0 ? null : timestamp,
    updatedAt: index === 0 ? "invalid-date" : timestamp
  }));
  vi.mocked(useAsyncValue).mockReturnValue({
    value: { status: "ready", conversations, totalDiscovered: 2000, successfullyParsed: 2000, skipped: 0, unsupported: 0 },
    loading: false, error: null, refresh: vi.fn(), setValue: vi.fn()
  });
  const formatter = vi.spyOn(Intl, "DateTimeFormat");
  const html = renderToStaticMarkup(<I18nProvider value={{ language, t: (key, values) => translate(language, key, values) }}><ConversationsPage /></I18nProvider>);
  expect(formatter).toHaveBeenCalledTimes(1);
  expect(html).toContain(expectedDate);
  expect(html).toContain(`<td>${translate(language, "common.dash")}</td><td>${translate(language, "common.dash")}</td>`);
  expect(html.match(/<tr>/g)).toHaveLength(2001);
});
