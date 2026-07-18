import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ServerStatus {
  online: boolean;
  players: number;
  max_players: number;
  version: string;
  ping_ms: number;
  tps: number;
}

export interface LauncherSettings {
  ram_gb: number;
  java_version: string;
  window_width: string;
  window_height: string;
  fullscreen: boolean;
  vsync: boolean;
  keep_open: boolean;
  debug_info: boolean;
  last_nickname: string;
  game_path: string;
}

export interface DownloadProgress {
  file: string;
  progress: number;
  downloaded_mb: number;
  total_mb: number;
  speed_mb_s: number;
}

export async function checkServerStatus(): Promise<ServerStatus> {
  return invoke<ServerStatus>('check_server_status');
}

export async function loadSettings(): Promise<LauncherSettings> {
  return invoke<LauncherSettings>('load_settings');
}

export async function saveSettings(settings: LauncherSettings): Promise<void> {
  return invoke('save_settings', { settings });
}

export async function downloadGame(gamePath: string): Promise<boolean> {
  return invoke<boolean>('download_game', { gamePath });
}

export async function launchGame(
  nickname: string,
  settings: LauncherSettings
): Promise<boolean> {
  return invoke<boolean>('launch_game', { nickname, settings });
}

export async function selectGameFolder(): Promise<string> {
  return invoke<string>('select_game_folder');
}

export async function windowMinimize(): Promise<void> {
  return invoke('window_minimize');
}

export async function windowMaximize(): Promise<void> {
  return invoke('window_maximize');
}

export async function windowClose(): Promise<void> {
  return invoke('window_close');
}

export async function windowIsMaximized(): Promise<boolean> {
  return invoke<boolean>('window_is_maximized');
}

export function onDownloadProgress(
  callback: (progress: DownloadProgress) => void
) {
  return listen<DownloadProgress>('download-progress', (event) => {
    callback(event.payload);
  });
}

export async function reportGameInventory(nickname: string): Promise<boolean> {
  return invoke<boolean>('report_game_inventory', { nickname });
}
