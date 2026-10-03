import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SkillsPage } from "./SkillsPage";

it("renders the local skills import and export workflow", () => {
  const html = renderToStaticMarkup(<SkillsPage />);
  expect(html).toContain("Import skill");
  expect(html).toContain("Local skills only");
});
