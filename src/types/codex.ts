export type OperatingSystem = "windows" | "macos" | "linux" | "unknown";

export interface CodexPaths {
  codexHome: string;
  configDir: string;
  backupDir: string;
  portableMode: boolean;
}

export interface DiagnosticsSnapshot {
  operatingSystem: OperatingSystem;
  architecture: string;
  codexHome: string;
  codexHomeExists: boolean;
  configDir: string;
  backupDir: string;
  codexCliVersion: string | null;
  skillsCount: number;
  petsCount: number;
}

export interface AppConfiguration {
  theme: "system" | "light" | "dark";
  portableMode: boolean;
  createSafetyBackups: boolean;
  language: "en" | "vi";
  logLevel: "error" | "warn" | "info" | "debug";
  codexEnvironments: CodexEnvironment[];
  fileTransferProfiles: FileTransferProfile[];
}

export type FileTransferMode = "simpleCopy" | "fastCopy" | "projectMigration" | "mirror";
export interface FileTransferConfig {
  source: string; destination: string; mode: FileTransferMode;
  includeSubfolders: boolean; preserveTimestamps: boolean; skipJunctionPoints: boolean; restartable: boolean; copyEmptyDirectories: boolean;
  verifyDestination: boolean; saveLog: boolean; shutdownWhenFinished: boolean;
  threads: 1 | 2 | 4 | 8 | 16 | 32; retries: number; retryWait: number;
  excludeFolders: string[]; excludeFiles: string[]; selectionEnabled: boolean; selectedEntries: string[]; mirrorConfirmed: boolean; systemLocationConfirmed: boolean; destinationDataConfirmed: boolean;
}
export interface FileTransferDirectoryEntry { name: string; path: string; isDirectory: boolean; isHidden: boolean; isSystem: boolean; }
export interface FileTransferDirectoryBreadcrumb { label: string; path: string; }
export interface FileTransferDirectoryListing { path: string; parent: string | null; breadcrumbs: FileTransferDirectoryBreadcrumb[]; entries: FileTransferDirectoryEntry[]; }
export interface FileTransferProfile { id: string; name: string; config: FileTransferConfig; }
export interface FileTransferReadiness { available: boolean; path: string | null; }
export interface FileTransferCommandPreview { command: string; arguments: string[]; warnings: string[]; }
export interface FileTransferOutput { line: string; stream: "stdout" | "stderr"; }
export interface FileTransferProgress { bytesCopied: number; totalBytes: number; percent: number; }
export interface FileTransferSummary { filesCopied: number | null; filesSkipped: number | null; filesFailed: number | null; bytesCopied: number | null; }
export interface FileTransferCompletion { phase: "analysis" | "transfer"; state: "completed" | "completed-with-warning" | "failed"; exitCode: number | null; interpretation: { status: "success" | "warning" | "error"; message: string }; summary: FileTransferSummary; durationSeconds: number; logPath: string | null; verification: "Verified" | "Differences found" | "Verification failed" | null; }
export interface FileTransferHistoryEntry { id: string; startedAt: string; completedAt: string; source: string; destination: string; preset: FileTransferMode; status: string; bytesCopied: number | null; filesCopied: number | null; durationSeconds: number; exitCode: number | null; logPath: string | null; }

export interface CodexEnvironment {
  id: string;
  displayName: string;
  commandAlias: string;
  codexHome: string;
  description: string | null;
  managesInstructions: boolean;
  launcherPath: string | null;
  createdAt: string;
  updatedAt: string;
}
export interface CodexEnvironmentInput {
  id?: string;
  displayName: string;
  commandAlias: string;
  codexHome: string;
  description?: string;
}
export interface CodexCliStatus { installed: boolean; path: string | null; version: string | null; }
export interface CodexEnvironmentStatus extends CodexEnvironment {
  isDefault: boolean;
  codexHomeExists: boolean;
  agentsExists: boolean;
  loginStatus: "loggedIn" | "notLoggedIn" | "unknown";
}
export interface CodexEnvironmentOverview {
  cli: CodexCliStatus;
  launcherDir: string;
  launcherDirExists: boolean;
  launcherDirInUserPath: boolean;
  environments: CodexEnvironmentStatus[];
}
export interface CodexEnvironmentInstructions { content: string; managed: boolean; }
export interface CodexEnvironmentActionResult { message: string; }

export type ConversationDiscoveryStatus = "ready" | "codexHomeMissing" | "sessionDirectoryMissing" | "permissionDenied" | "filesystemUnavailable";

export interface ConversationSummary {
  id: string;
  title: string | null;
  createdAt: string | null;
  updatedAt: string | null;
  projectPath: string | null;
  projectName: string | null;
  source: string | null;
}

export interface ConversationDiscovery {
  status: ConversationDiscoveryStatus;
  conversations: ConversationSummary[];
  totalDiscovered: number;
  successfullyParsed: number;
  skipped: number;
  unsupported: number;
}

export interface BackupSession {
  id: string;
  title: string | null;
  createdAt: string | null;
  updatedAt: string | null;
  archivePath: string;
  bytes: number;
}

export interface BackupPreview {
  formatVersion: number;
  sessionCount: number;
  totalBytes: number;
  backupDirectory: string;
  sessions: BackupSession[];
}

export interface BackupResult {
  archivePath: string;
  sessionCount: number;
  totalBytes: number;
}

export interface ArchiveInspection {
  archiveName: string;
  createdAt: string | null;
  platform: string | null;
  codexCliVersion: string | null;
  sessionCount: number;
  totalBytes: number;
  validation: { valid: boolean; formatVersion: number | null };
  warnings: string[];
  errors: string[];
  restoreToken: string | null;
  sessions: BackupSession[];
}
export interface RestorePreview { sessionCount: number; totalBytes: number; destinationRoot: string; sessions: Array<{ id: string; archivePath: string; destinationPath: string; bytes: number; conflict: boolean }>; conflictCount: number; plannedCreates: number; safetyBackupWillBeCreated: boolean; }
export interface RestoreResult { restoredCount: number; skippedConflicts: number; totalBytes: number; safetyBackupPath: string | null; }
export interface DeletePreview { formatVersion: number; sessionCount: number; totalBytes: number; quarantineDirectory: string; sessions: BackupSession[]; localOnly: boolean; }
export type DeleteOutcome = "completed" | "partial" | "rolledBack" | "failed";
export interface DeleteResult { outcome: DeleteOutcome; deletedCount: number; restoredCount: number; skippedCount: number; totalBytes: number; safetyArchivePath: string | null; }
export type RestoreOutcome = "completed" | "partial" | "rolledBack" | "failed";
export type HistoryAction = "restore" | "delete";
export interface RestoreHistoryEntry {
  occurredAt: string;
  action: HistoryAction;
  archiveName: string;
  sessionIds: string[];
  deletedCount: number;
  restoredCount: number;
  skippedConflicts: number;
  safetyBackupPath: string | null;
  outcome: RestoreOutcome;
  errorCode: string | null;
}

export interface BeyondCompareReadiness { supported: boolean; secretExportAcknowledgementRequired: boolean; }
export interface BeyondCompareBundlePreview { token: string; packageName: string; bytes: number; sensitive: boolean; }
export interface BeyondCompareBundleInspection { token: string; bundleName: string; createdAt: string; bytes: number; sensitive: boolean; }
export interface BeyondCompareRecoveryPreview { token: string; packageName: string; bytes: number; stagingPath: string; }
export interface SourceTreeReadiness { supported: boolean; bookmarksFound: boolean; }
export interface SourceTreePreview { token: string; sourceAppVersion: string; bookmarkCount: number; repositoryPaths: string[]; bytes: number; sensitive: boolean; }
export interface SourceTreeInspection { token: string; bundleName: string; createdAt: string; sourceAppVersion: string; bookmarkCount: number; bytes: number; sensitive: boolean; }
export interface SourceTreeRecoveryPreview { token: string; stagingPath: string; destinationPath: string; destinationConflict: boolean; manualOnly: boolean; }
export interface XamppReadiness { supported: boolean; installationFound: boolean; stopped: boolean; }
export interface XamppPreview { token: string; sourceAppVersion: string; architecture: string; projects: string[]; configFiles: string[]; fileCount: number; bytes: number; excludedCount: number; sensitive: boolean; }
export interface XamppInspection { token: string; bundleName: string; createdAt: string; sourceAppVersion: string; architecture: string; projects: string[]; fileCount: number; bytes: number; sensitive: boolean; }
export interface XamppRecoveryPreview { token: string; stagingPath: string; destinationConflicts: number; manualOnly: boolean; }
