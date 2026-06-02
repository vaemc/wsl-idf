import * as vscode from "vscode";

const SECTION = "wsl-idf";

export interface WslIdfConfig {
  executable: string;
  port: string;
  monitorBaud: number;
  autoMonitor: boolean;
  terminalName: string;
  reuseActiveTerminal: boolean;
  statusBarPriority: number;
  statusBarCompact: boolean;
}

export function readConfig(): WslIdfConfig {
  const c = vscode.workspace.getConfiguration(SECTION);
  return {
    executable: c.get<string>("executable", "wsl-idf"),
    port: c.get<string>("port", ""),
    monitorBaud: c.get<number>("monitorBaud", 115200),
    autoMonitor: c.get<boolean>("autoMonitor", true),
    terminalName: c.get<string>("terminalName", "wsl-idf"),
    reuseActiveTerminal: c.get<boolean>("reuseActiveTerminal", false),
    statusBarPriority: c.get<number>("statusBar.priority", 100),
    statusBarCompact: c.get<boolean>("statusBar.compact", false),
  };
}

export async function updateConfig<T>(
  key: string,
  value: T,
  target: vscode.ConfigurationTarget = vscode.ConfigurationTarget.Workspace,
): Promise<void> {
  const c = vscode.workspace.getConfiguration(SECTION);
  // Workspace 不可写时回退到 Global
  try {
    await c.update(key, value, target);
  } catch {
    await c.update(key, value, vscode.ConfigurationTarget.Global);
  }
}
