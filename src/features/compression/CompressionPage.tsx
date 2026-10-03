import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import { cancelCompression, getCompressionReadiness, pickCompressionFolder, previewCompression, scanCompressionSource, startCompression } from "../../services/tauri";
import type { CompressionCommandPreview, CompressionCompletion, CompressionConfig, CompressionOutput, CompressionProgress, CompressionReadiness, CompressionSourceTree } from "../../types/codex";
import "./CompressionPage.css";

const initial: CompressionConfig = { source: "", outputFolder: "", mode: "fast", excludedPaths: [], regexExclusions: [] };
const treePageSize = 500;

export function CompressionPage() {
  const [readiness, setReadiness] = useState<CompressionReadiness | null>(null);
  const [tree, setTree] = useState<CompressionSourceTree | null>(null);
  const [config, setConfig] = useState(initial);
  const [patterns, setPatterns] = useState("");
  const [preview, setPreview] = useState<CompressionCommandPreview | null>(null);
  const [status, setStatus] = useState<"idle" | "running" | "cancelling" | "completed" | "failed" | "cancelled">("idle");
  const [completion, setCompletion] = useState<CompressionCompletion | null>(null);
  const [output, setOutput] = useState<string[]>([]);
  const [error, setError] = useState("");
  const [sourceLoading, setSourceLoading] = useState(false);
  const [preparing, setPreparing] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [treeFilter, setTreeFilter] = useState("");
  const [treeLimit, setTreeLimit] = useState(treePageSize);
  const busy = status === "running" || status === "cancelling" || sourceLoading || preparing;
  const canCancel = status === "running" && !preparing;
  const filteredEntries = useMemo(() => {
    const query = treeFilter.trim().toLocaleLowerCase();
    return query ? tree?.entries.filter((entry) => entry.path.toLocaleLowerCase().includes(query)) ?? [] : tree?.entries ?? [];
  }, [tree, treeFilter]);
  const visibleEntries = filteredEntries.slice(0, treeLimit);

  useEffect(() => {
    let disposed = false;
    const unlisten: Array<() => void> = [];
    void Promise.all([
      listen<CompressionOutput>("compression-output", ({ payload }) => {
        if (!disposed) setOutput((previous) => [...previous.slice(-399), `${payload.stream === "stderr" ? "! " : ""}${payload.line}`]);
      }),
      listen<CompressionCompletion>("compression-completed", ({ payload }) => {
        if (!disposed) { setCompletion(payload); setStatus(payload.state); setPreparing(false); if (payload.state === "completed") setProgress(100); if (payload.state !== "completed") setError(payload.message); }
      }),
      listen<CompressionProgress>("compression-progress", ({ payload }) => {
        if (!disposed) { setProgress(payload.percent); setPreparing(false); }
      }),
    ]).then(async (listeners) => {
      if (disposed) { listeners.forEach((listener) => listener()); return; }
      unlisten.push(...listeners);
      const value = await getCompressionReadiness();
      if (!disposed) { setReadiness(value); if (value.running) setStatus((current) => current === "idle" ? "running" : current); }
    }).catch((reason) => { if (!disposed) setError(String(reason)); });
    return () => { disposed = true; unlisten.forEach((listener) => listener()); };
  }, []);

  const chooseSource = async () => {
    setError("");
    try {
      const path = await pickCompressionFolder("Choose source folder");
      if (!path) return;
      setSourceLoading(true);
      setTree(null);
      setConfig({ ...initial, source: path, outputFolder: "" });
      const next = await scanCompressionSource(path);
      setTree(next);
      setConfig({ ...initial, source: next.source, outputFolder: next.defaultOutputFolder });
      setPatterns("");
      setPreview(null);
      setCompletion(null);
      setOutput([]);
      setProgress(null);
      setTreeFilter("");
      setTreeLimit(treePageSize);
      setStatus("idle");
    } catch (reason) { setError(String(reason)); }
    finally { setSourceLoading(false); }
  };

  const chooseOutput = async () => {
    try {
      const path = await pickCompressionFolder("Choose output folder");
      if (path) { setConfig((current) => ({ ...current, outputFolder: path })); setPreview(null); }
    } catch (reason) { setError(String(reason)); }
  };

  const toggle = (path: string) => {
    setConfig((current) => ({ ...current, excludedPaths: current.excludedPaths.includes(path) ? current.excludedPaths.filter((value) => value !== path) : [...current.excludedPaths, path] }));
    setPreview(null);
  };

  const effectiveConfig = (): CompressionConfig => ({ ...config, regexExclusions: patterns.split(/\r?\n/).filter((value) => value.trim()) });

  const inspect = async () => {
    setError("");
    setPreparing(true);
    try { setPreview(await previewCompression(effectiveConfig())); }
    catch (reason) { setPreview(null); setError(String(reason)); }
    finally { setPreparing(false); }
  };

  const start = async () => {
    setError(""); setOutput([]); setCompletion(null); setProgress(null); setPreparing(true); setStatus("running");
    try { await startCompression(effectiveConfig()); }
    catch (reason) { setStatus("failed"); setError(String(reason)); }
    finally { setPreparing(false); }
  };

  const cancel = async () => {
    setStatus((current) => current === "running" ? "cancelling" : current);
    try { await cancelCompression(); }
    catch (reason) { setError(String(reason)); setStatus((current) => current === "cancelling" ? "running" : current); }
  };

  const isExcluded = (path: string) => config.excludedPaths.some((value) => path === value || path.startsWith(`${value}/`));

  const statusText = sourceLoading ? "Scanning the source tree…" : preparing ? "Preparing selected files for 7-Zip…" : status === "running" && progress != null ? `Compressing… ${progress}%` : status === "running" ? "Waiting for 7-Zip progress…" : status;

  return <div className="compression-page">
    <header className="page-header"><div><h1>File Compression</h1><p>Create a 7z archive from one folder. Review the read-only tree, exclude relative paths or regex matches, then choose a compression level.</p></div></header>
    <section className="file-transfer-card"><h2>7-Zip readiness</h2><p>{readiness?.message ?? "Checking 7-Zip…"}</p>{readiness?.path && <code>{readiness.path}</code>}</section>
    <section className="file-transfer-card"><h2>Folders</h2><div className="compression-folder"><span>Source</span><code>{tree?.source || config.source || "No folder selected"}</code><button disabled={busy} type="button" onClick={() => void chooseSource()}>Choose source…</button></div><div className="compression-folder"><span>Output</span><code>{config.outputFolder || "Choose source first"}</code><button disabled={busy || !tree} type="button" onClick={() => void chooseOutput()}>Choose output…</button></div>{sourceLoading && <p className="compression-working"><progress aria-label="Scanning source tree" />Scanning the source tree. Large folders can take a moment.</p>}<small>The default output is the source parent. The archive is named after the source folder.</small></section>
    {tree && <section className="file-transfer-card"><h2>Source tree</h2><p>{tree.entries.length} items · {tree.skippedReparsePoints} reparse points skipped. Uncheck a folder to exclude everything below it.</p><label className="compression-tree-filter">Find a path<input value={treeFilter} disabled={busy} placeholder="node_modules, cache, logs…" onChange={(event) => { setTreeFilter(event.target.value); setTreeLimit(treePageSize); }} /></label><p className="compression-tree-summary">Showing {visibleEntries.length.toLocaleString()} of {filteredEntries.length.toLocaleString()} matching items.</p><div className="compression-tree" role="tree" aria-label="Source items" aria-busy={sourceLoading}>{visibleEntries.map((entry) => <label key={entry.path} role="treeitem" aria-level={entry.path.split("/").length} style={{ paddingLeft: Math.min(entry.path.split("/").length - 1, 12) * 18 }}><input type="checkbox" disabled={busy || config.excludedPaths.some((value) => entry.path.startsWith(`${value}/`))} checked={!isExcluded(entry.path)} onChange={() => toggle(entry.path)} /><span>{entry.isDirectory ? "📁" : "📄"} {entry.path.split("/").at(-1)}</span><small>{entry.path}</small></label>)}</div>{visibleEntries.length < filteredEntries.length && <button className="compression-show-more" disabled={busy} type="button" onClick={() => setTreeLimit((current) => current + treePageSize)}>Show 500 more</button>}<label className="compression-patterns">Regex exclusions, one Rust regex per line, matched against slash-normalized paths<textarea disabled={busy} value={patterns} onChange={(event) => { setPatterns(event.target.value); setPreview(null); }} placeholder={"(?i)\\.log$\n^build/"} /></label></section>}
    <section className="file-transfer-card"><h2>Archive</h2><fieldset disabled={busy || !tree}><legend>Compression</legend><label><input type="radio" checked={config.mode === "fast"} onChange={() => { setConfig({ ...config, mode: "fast" }); setPreview(null); }} /> Fast (-mx=1)</label><label><input type="radio" checked={config.mode === "strong"} onChange={() => { setConfig({ ...config, mode: "strong" }); setPreview(null); }} /> Strong (-mx=9)</label></fieldset><div className="file-transfer-actions"><button type="button" disabled={!tree || busy} onClick={() => void inspect()}>Preview</button><button type="button" disabled={!readiness?.available || !preview || busy} onClick={() => void start()}>Create archive</button><button type="button" disabled={!canCancel} onClick={() => void cancel()}>{status === "cancelling" ? "Cancelling…" : "Cancel"}</button></div>{preview && <><p>{preview.includedFiles} files · {preview.includedFolders} empty folders · {preview.skippedReparsePoints} reparse points skipped</p><code className="file-transfer-command">{preview.command}</code><p>Output: <code>{preview.archivePath}</code></p></>}</section>
    <section className="file-transfer-card" aria-live="polite"><h2>Status</h2><p>{statusText}</p>{progress != null && status === "running" ? <div className="compression-progress"><progress value={progress} max="100">{progress}%</progress><strong>{progress}%</strong></div> : busy && <progress aria-label="Indeterminate compression progress" />}{completion && <p>{completion.message}</p>}{completion?.archivePath && <p>Created: <code>{completion.archivePath}</code></p>}{error && <p role="alert" className="error-banner">{error}</p>}<pre className="file-transfer-output">{output.join("\n")}</pre></section>
  </div>;
}
