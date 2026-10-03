import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { SettingsPage } from "./SettingsPage";

it("shows a configuration load failure instead of leaving settings in a loading state", () => {
  const html = renderToStaticMarkup(<SettingsPage configuration={null} loadError="Configuration file is invalid." onSave={async () => {}} />);
  expect(html).toContain("Configuration file is invalid.");
  expect(html).not.toContain("Loading settings…");
});
