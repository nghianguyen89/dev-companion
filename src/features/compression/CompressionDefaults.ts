import type { CompressionConfig, CompressionTreeEntry } from "../../types/codex";

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

export const compressionPresets: Array<{ id: "project" | "all" | "cache"; mode: "fast" | "strong"; excludedDirectoryNames: string[]; excludedFileExtensions: string[] }> = [
  { id: "project", mode: "fast", excludedDirectoryNames: [...defaultExcludedDirectoryNames], excludedFileExtensions: [".log", ".tmp"] },
  { id: "all", mode: "fast", excludedDirectoryNames: [], excludedFileExtensions: [] },
  { id: "cache", mode: "fast", excludedDirectoryNames: [...defaultExcludedDirectoryNames].filter((name) => !["node_modules", "vendor", "bower_components", "build", "dist", "target"].includes(name)), excludedFileExtensions: [".log", ".tmp"] },
];

export function compressionPathExcluded(config: CompressionConfig, entry: CompressionTreeEntry) {
  const path = entry.path.toLowerCase();
  const within = (parent: string) => path === parent.toLowerCase() || path.startsWith(parent.toLowerCase() + "/");
  if (config.excludedPaths.some(within)) return true;
  if (config.includedPaths?.some(within)) return false;
  const parts = path.split("/");
  const folders = entry.isDirectory ? parts : parts.slice(0, -1);
  return folders.some((name) => config.excludedDirectoryNames?.some((value) => value.toLowerCase() === name))
    || (!entry.isDirectory && !!config.excludedFileExtensions?.some((extension) => path.endsWith(extension.toLowerCase())));
}

export function toggleCompressionPath(config: CompressionConfig, entry: CompressionTreeEntry): CompressionConfig {
  const excludedPaths = config.excludedPaths.filter((path) => path !== entry.path);
  const includedPaths = (config.includedPaths ?? []).filter((path) => path !== entry.path);
  return compressionPathExcluded(config, entry)
    ? { ...config, excludedPaths, includedPaths: [...includedPaths, entry.path] }
    : { ...config, excludedPaths: [...excludedPaths, entry.path], includedPaths };
}
