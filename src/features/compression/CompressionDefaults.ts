import type { CompressionTreeEntry } from "../../types/codex";

const defaultExcludedDirectoryNames = new Set([".cache", ".gradle", ".mypy_cache", ".next", ".npm", ".nuxt", ".parcel-cache", ".pnpm-store", ".pytest_cache", ".turbo", ".vite", ".yarn", "__pycache__", "bower_components", "build", "cache", "coverage", "dist", "log", "logs", "node_modules", "target", "temp", "tmp", "vendor"]);

export function defaultCompressionExcludedPaths(entries: CompressionTreeEntry[]) {
  const excluded: string[] = [];
  for (const entry of entries) {
    const name = entry.path.split("/").at(-1)?.toLowerCase() ?? "";
    const matches = entry.isDirectory ? defaultExcludedDirectoryNames.has(name) : name.endsWith(".log") || name.endsWith(".tmp");
    if (matches && !excluded.some((parent) => entry.path.startsWith(`${parent}/`))) excluded.push(entry.path);
  }
  return excluded;
}
