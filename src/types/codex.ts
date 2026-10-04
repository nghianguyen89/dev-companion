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
export interface BackupStorageEntry { name: string; bytes: number; modifiedAt: number | null; }
export interface BackupStorageSummary { directory: string; fileCount: number; totalBytes: number; recentFiles: BackupStorageEntry[]; }
export interface BackupStorageOverview { sessionBackups: BackupStorageSummary; personalBundles: BackupStorageSummary; }

export interface CodexSkill { id: string; manifestBytes: number; }
export interface CodexSkillsOverview { directory: string; skills: CodexSkill[]; skipped: number; }
export interface CodexPet { id: string; displayName: string; description: string; spriteVersionNumber: number; spriteFile: string; }
export interface CodexPetsOverview { directory: string; pets: CodexPet[]; skipped: number; }
export interface CodexContentActionResult { path: string; }

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
export interface CompressionReadiness { available: boolean; path: string | null; message: string; running: boolean; }
export interface CompressionTreeEntry { path: string; isDirectory: boolean; bytes: number; }
export interface CompressionSourceTree { source: string; defaultOutputFolder: string; entries: CompressionTreeEntry[]; skippedReparsePoints: number; }
export interface CompressionConfig { source: string; outputFolder: string; mode: "fast" | "strong"; excludedPaths: string[]; regexExclusions: string[]; }
export interface CompressionCommandPreview { command: string; archivePath: string; includedFiles: number; includedFolders: number; skippedReparsePoints: number; }
export interface CompressionOutput { line: string; stream: "stdout" | "stderr"; }
export interface CompressionProgress { percent: number; }
export interface CompressionCompletion { state: "completed" | "failed" | "cancelled"; archivePath: string | null; exitCode: number | null; message: string; }

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
export type CodexMigrationGroup = "chat" | "settings" | "skills" | "pets" | "worktrees" | "plugins" | "visualizations";
export interface CodexMigrationAccount { id: string; label: string; folder: string; }
export interface CodexMigrationEntry { accountId: string; relativePath: string; archivePath: string; group: CodexMigrationGroup; bytes: number; sha256: string; }
export interface CodexMigrationGroupSummary { id: string; files: number; bytes: number; reason: string; }
export interface CodexMigrationAccountEstimate { accountId: string; groups: CodexMigrationGroupSummary[]; }
export interface CodexMigrationOverview { accounts: CodexMigrationAccount[]; estimates: CodexMigrationAccountEstimate[]; }
export interface CodexMigrationPreview { token: string; accounts: CodexMigrationAccount[]; groups: CodexMigrationGroupSummary[]; excluded: CodexMigrationGroupSummary[]; entries: CodexMigrationEntry[]; }
export interface CodexMigrationCreated { archivePath: string; archiveBytes: number; files: number; }
export interface CodexMigrationArchive { name: string; bytes: number; modifiedAt: number | null; }
export interface CodexMigrationArchives { directory: string; archives: CodexMigrationArchive[]; totalBytes: number; }
export interface CodexMigrationRestorePreview { token: string; items: Array<{ accountLabel: string; path: string; archivePath: string; status: "new" | "identical" | "conflict"; bytes: number }>; }
export interface CodexMigrationRestored { restored: number; skipped: number; errors: string[]; rollbackRemaining: number; }

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

export interface DeleteSession {
  id: string;
  title: string | null;
  createdAt: string | null;
  updatedAt: string | null;
  archivePath: string;
  bytes: number;
}

export interface DeletePreview { formatVersion: number; sessionCount: number; totalBytes: number; quarantineDirectory: string; sessions: DeleteSession[]; localOnly: boolean; }
export type DeleteOutcome = "completed" | "partial" | "rolledBack" | "failed";
export interface DeleteResult { outcome: DeleteOutcome; deletedCount: number; restoredCount: number; skippedCount: number; totalBytes: number; safetyArchivePath: string | null; }

export interface BeyondCompareReadiness { supported: boolean; secretExportAcknowledgementRequired: boolean; }
export interface BeyondCompareBundlePreview { token: string; packageName: string; bytes: number; sensitive: boolean; }
export interface BeyondCompareBundleInspection { token: string; bundleName: string; createdAt: string; bytes: number; sensitive: boolean; }
export interface BeyondCompareRecoveryPreview { token: string; packageName: string; bytes: number; stagingPath: string; }
export interface SourceTreeReadiness { supported: boolean; bookmarksFound: boolean; }
export interface SourceTreePreview { token: string; sourceAppVersion: string; bookmarkCount: number; repositoryPaths: string[]; bytes: number; sensitive: boolean; }
export interface SourceTreeInspection { token: string; bundleName: string; createdAt: string; sourceAppVersion: string; bookmarkCount: number; bytes: number; sensitive: boolean; }
export interface SourceTreeRecoveryPreview { token: string; stagingPath: string; destinationPath: string; destinationConflict: boolean; manualOnly: boolean; }
export interface SourceTreeConfigFile { name: string; bytes: number; }
export interface SourceTreeConfigPreview { token: string; files: SourceTreeConfigFile[]; missing: string[]; bytes: number; sensitive: boolean; }
export interface SourceTreeConfigInspection { token: string; bundleName: string; files: SourceTreeConfigFile[]; bytes: number; sensitive: boolean; }
export interface SourceTreeConfigRecoveryPreview { token: string; files: SourceTreeConfigFile[]; existingTargets: number; }
export interface SourceTreeConfigRecoveryResult { restored: number; safetyCopyPath: string; }
export interface XamppReadiness { supported: boolean; installationFound: boolean; stopped: boolean; }
export interface XamppPreview { token: string; sourceAppVersion: string; architecture: string; projects: string[]; configFiles: string[]; fileCount: number; bytes: number; excludedCount: number; sensitive: boolean; }
export interface XamppInspection { token: string; bundleName: string; createdAt: string; sourceAppVersion: string; architecture: string; projects: string[]; fileCount: number; bytes: number; sensitive: boolean; }
export interface XamppRecoveryPreview { token: string; stagingPath: string; destinationConflicts: number; manualOnly: boolean; }
