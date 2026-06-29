import * as vscode from "vscode";
import { readConfig } from "./config";

interface ItemDef {
  /** 唯一 ID，用于内部识别 */
  id: string;
  /** 显示文本（含图标 $(...)） */
  text: () => string;
  tooltip: string;
  command: string;
  /** 紧凑模式下是否保留 */
  keepInCompact?: boolean;
}

const ITEMS: ItemDef[] = [
  {
    id: "port",
    text: () => {
      const port = readConfig().port;
      return port ? `$(plug) ${port}` : "$(plug) 选择端口";
    },
    tooltip: "点击选择 Windows COM 端口",
    command: "wsl-idf.selectPort",
    keepInCompact: true,
  },
  {
    id: "menu",
    text: () => "$(rocket) wsl-idf",
    tooltip: "wsl-idf 快捷菜单（Flash / Monitor / 设置等）",
    command: "wsl-idf.showMenu",
    keepInCompact: true,
  },
  {
    id: "build",
    text: () => "$(tools) Build",
    tooltip: "idf.py build",
    command: "wsl-idf.build",
  },
  {
    id: "flash",
    text: () => "$(zap) Flash",
    tooltip: "wsl-idf -c flash（自动 build 全部分区烧录）",
    command: "wsl-idf.flash",
  },
  {
    id: "flashApp",
    text: () => "$(rocket) App",
    tooltip: "wsl-idf -c flash-app（仅烧录 app 分区）",
    command: "wsl-idf.flashApp",
  },
  {
    id: "flashErase",
    text: () => "$(flame) Erase+Flash",
    tooltip: "wsl-idf -c flash-erase（擦除整片后烧录）",
    command: "wsl-idf.flashErase",
  },
  {
    id: "erase",
    text: () => "$(trash) Erase",
    tooltip: "wsl-idf -c erase（仅擦除 Flash）",
    command: "wsl-idf.erase",
  },
  {
    id: "merge",
    text: () => "$(file-binary) Merge",
    tooltip: "wsl-idf -c merge（合并固件为 full-*.bin）",
    command: "wsl-idf.merge",
  },
  {
    id: "extractBins",
    text: () => "$(export) Extract",
    tooltip: "wsl-idf -c extract-bins（提取分区 bin 到 flash-bins/）",
    command: "wsl-idf.extractBins",
  },
  {
    id: "monitor",
    text: () => `$(terminal) Monitor`,
    tooltip: "wsl-idf monitor（按 Ctrl+C 退出）",
    command: "wsl-idf.monitor",
  },
];

export class WslIdfStatusBar implements vscode.Disposable {
  private items: vscode.StatusBarItem[] = [];

  build(context: vscode.ExtensionContext): void {
    this.dispose();

    const cfg = readConfig();
    const compact = cfg.statusBarCompact;
    const basePriority = cfg.statusBarPriority;

    // priority 数值越大越靠左，因此从基础值递减保证按数组顺序显示
    let prio = basePriority;
    for (const def of ITEMS) {
      if (compact && !def.keepInCompact) {
        continue;
      }
      const item = vscode.window.createStatusBarItem(
        vscode.StatusBarAlignment.Left,
        prio--,
      );
      item.text = def.text();
      item.tooltip = def.tooltip;
      item.command = def.command;
      item.show();
      this.items.push(item);
      context.subscriptions.push(item);
    }
  }

  /** 刷新文本（端口变化等场景）。 */
  refresh(): void {
    for (let i = 0; i < this.items.length; i++) {
      const def = ITEMS[i];
      if (!def) continue;
      this.items[i].text = def.text();
    }
  }

  dispose(): void {
    for (const item of this.items) {
      item.dispose();
    }
    this.items = [];
  }
}
