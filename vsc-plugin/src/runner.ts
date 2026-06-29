import * as vscode from "vscode";
import { readConfig } from "./config";

function getOrCreateTerminal(): vscode.Terminal {
  const cfg = readConfig();
  if (cfg.reuseActiveTerminal && vscode.window.activeTerminal) {
    return vscode.window.activeTerminal;
  }
  const name = cfg.terminalName || "wsl-idf";
  const existing = vscode.window.terminals.find((t) => t.name === name);
  if (existing) {
    return existing;
  }
  return vscode.window.createTerminal({ name });
}

export interface RunOptions {
  /** 是否在末尾自动追加 `monitor [baud]`。仅在配置允许且命令属于烧录/擦除类时调用方传 true。 */
  appendMonitor?: boolean;
}

/**
 * 构造完整的 wsl-idf 命令行并发送到终端执行。
 *
 * @param subcommand 比如 "flash" / "flash-app" / "erase" / "merge"；null 表示仅 monitor。
 * @param opts.appendMonitor 是否追加 `monitor <baud>`。
 */
export function runWslIdf(
  subcommand: string | null,
  opts: RunOptions = {},
): void {
  const cfg = readConfig();
  const exe = cfg.executable || "wsl-idf";

  const parts: string[] = [exe];
  if (subcommand) {
    parts.push("-c", subcommand);
  }

  const needsPort =
    subcommand !== null &&
    subcommand !== "merge" &&
    subcommand !== "extract-bins" &&
    subcommand !== "about";
  if (needsPort || opts.appendMonitor) {
    if (!cfg.port) {
      void vscode.window
        .showWarningMessage(
          "wsl-idf: 尚未选择 COM 口。点击状态栏端口按钮选择。",
          "立即选择",
        )
        .then((choice) => {
          if (choice === "立即选择") {
            void vscode.commands.executeCommand("wsl-idf.selectPort");
          }
        });
      return;
    }
    parts.push("-p", cfg.port);
  }

  if (opts.appendMonitor) {
    parts.push("monitor", String(cfg.monitorBaud || 115200));
  }

  const cmd = parts.join(" ");
  const term = getOrCreateTerminal();
  term.show(true);
  term.sendText(cmd);
}

/** 在终端中运行任意命令（用于 idf.py build/menuconfig/clean 等非 wsl-idf 命令）。 */
export function runRaw(cmd: string): void {
  const term = getOrCreateTerminal();
  term.show(true);
  term.sendText(cmd);
}
