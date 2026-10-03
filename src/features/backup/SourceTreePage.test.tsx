import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SourceTreePage } from "./SourceTreePage";

it("requires an encrypted SourceTree configuration bundle password", () => {
  const html = renderToStaticMarkup(<SourceTreePage />);
  expect(html).toContain("SourceTree");
  expect(html).toContain("configuration");
  expect(html).toContain("Create encrypted bundle");
  expect(html).toContain("New bundle password");
  expect(html).toContain("Windows");
});
