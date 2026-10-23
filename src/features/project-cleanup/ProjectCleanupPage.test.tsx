import { expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { ProjectCleanupCandidates, ProjectCleanupPage } from "./ProjectCleanupPage";

it("requires scan and preview before offering a permanent delete action", () => {
  const html = renderToStaticMarkup(<ProjectCleanupPage />);
  expect(html).toContain("Git or SVN is not required");
  expect(html).toContain("lockfiles");
  expect(html).not.toContain("Delete selected generated files</button>");
  expect(html).not.toContain("Type DELETE GENERATED");
});
it("shows generated paths without selecting them and disables blocked trees", () => {
  const html = renderToStaticMarkup(<ProjectCleanupCandidates candidates={[
    { id: "one", path: "dự án/node_modules", kind: "nodeDependencies", files: 10, bytes: 1024, blocked: null },
    { id: "two", path: ".parcel-cache", kind: "nodeCache", files: 1, bytes: 12, blocked: "Junction detected" },
  ]} selected={[]} disabled={false} onToggle={() => {}} />);
  expect(html).toContain("dự án/node_modules");
  expect(html).toContain("Junction detected");
  expect(html).not.toContain("checked=");
  expect(html.match(/disabled=""/g)).toHaveLength(1);
});

it("offers both cleanup methods and defaults to the recycle bin", () => {
  const html = renderToStaticMarkup(<ProjectCleanupPage />);
  expect(html).toContain('value="recycle" selected=""');
  expect(html).toContain("Move to Recycle Bin");
  expect(html).toContain("Delete permanently");
});
