import { invoke } from "@tauri-apps/api/core";
import type { Config, HistoryEntry, MoveOutcome, QueueItem, Status } from "./types";

export const getConfig = () => invoke<Config>("get_config");
export const setConfig = (config: Config) => invoke<void>("set_config", { config });
export const listQueue = () => invoke<QueueItem[]>("list_queue");
export const listHistory = () => invoke<HistoryEntry[]>("list_history");
export const getStatus = () => invoke<Status>("get_status");
export const dismiss = (path: string) => invoke<void>("dismiss", { path });
export const undo = (id: string) => invoke<HistoryEntry>("undo", { id });

export const confirmMove = (args: {
  path: string;
  customDestination?: string | null;
  remember: boolean;
  applyAll: boolean;
}) =>
  invoke<MoveOutcome[]>("confirm_move", {
    path: args.path,
    customDestination: args.customDestination ?? null,
    remember: args.remember,
    applyAll: args.applyAll,
  });

export function formatBytes(bytes: number): string {
  if (!bytes) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const index = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, index);
  return `${value.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

export function baseName(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}