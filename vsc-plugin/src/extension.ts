import * as vscode from "vscode";
import { readConfig, updateConfig } from "./config";
import { listSerialPorts } from "./ports";
import { runRaw, runWslIdf } from "./runner";
import { WslIdfStatusBar } from "./statusBar";

let statusBar: WslIdfStatusBar;

export function activate(context: vscode.ExtensionContext): void {
  statusBar = new WslIdfStatusBar();
  statusBar.build(context);

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("wsl-idf")) {
        statusBar.build(context);
      }
    }),
  );

  registerCommands(context);
  context.subscriptions.push(statusBar);
}

export function deactivate(): void {
  statusBar?.dispose();
}

function registerCommands(context: vscode.ExtensionContext): void {
  const reg = (id: string, fn: (...args: unknown[]) => unknown) =>
    context.subscriptions.push(vscode.commands.registerCommand(id, fn));

  reg("wsl-idf.selectPort", selectPort);
  reg("wsl-idf.refreshPorts", refreshPorts);
  reg("wsl-idf.setBaud", setBaud);
  reg("wsl-idf.toggleAutoMonitor", toggleAutoMonitor);

  reg("wsl-idf.build", () => runRaw("idf.py build"));
  reg("wsl-idf.menuconfig", () => runRaw("idf.py menuconfig"));
  reg("wsl-idf.clean", () => runRaw("idf.py clean"));
  reg("wsl-idf.fullclean", () => runRaw("idf.py fullclean"));

  reg("wsl-idf.flash", () => flashCommand("flash"));
  reg("wsl-idf.flashApp", () => flashCommand("flash-app"));
  reg("wsl-idf.flashErase", () => flashCommand("flash-erase"));
  reg("wsl-idf.erase", () => flashCommand("erase"));
  reg("wsl-idf.merge", () => runWslIdf("merge"));
  reg("wsl-idf.extractBins", () => runWslIdf("extract-bins"));
  reg("wsl-idf.monitor", () =>
    runWslIdf(null, { appendMonitor: true }),
  );
  reg("wsl-idf.about", () => runRaw(`${readConfig().executable} --about`));

  reg("wsl-idf.showMenu", showQuickMenu);
}

function flashCommand(sub: "flash" | "flash-app" | "flash-erase" | "erase"): void {
  const cfg = readConfig();
  runWslIdf(sub, { appendMonitor: cfg.autoMonitor });
}

async function selectPort(): Promise<void> {
  const current = readConfig().port;
  const enumeration = await vscode.window.withProgress(
    { location: vscode.ProgressLocation.Window, title: "wsl-idf: 枚举 COM 口..." },
    () => listSerialPorts(),
  );

  type Item = vscode.QuickPickItem & { value?: string; action?: "manual" };

  const items: Item[] = enumeration.ports.map((p) => ({
    label: p,
    description: p === current ? "（当前）" : undefined,
    value: p,
  }));

  if (enumeration.error) {
    items.push({
      label: "$(warning) 枚举失败",
      detail: enumeration.error,
      kind: vscode.QuickPickItemKind.Separator,
    });
  }
  if (items.length === 0) {
    items.push({
      label: "$(info) 未发现 COM 口",
      detail: "请确认设备已连接且驱动正常",
      kind: vscode.QuickPickItemKind.Separator,
    });
  }
  items.push({ label: "", kind: vscode.QuickPickItemKind.Separator });
  items.push({
    label: "$(edit) 手动输入...",
    description: "输入自定义 COM 名称",
    action: "manual",
  });
  items.push({
    label: "$(refresh) 刷新列表",
    action: "manual",
    value: "__refresh__",
  });

  const picked = await vscode.window.showQuickPick(items, {
    title: "选择 wsl-idf 使用的 Windows COM 端口",
    placeHolder: current ? `当前：${current}` : "未选择",
  });
  if (!picked) return;

  if (picked.value === "__refresh__") {
    await selectPort();
    return;
  }
  if (picked.action === "manual") {
    const input = await vscode.window.showInputBox({
      title: "手动输入 COM 端口",
      value: current,
      placeHolder: "例如 COM3",
      validateInput: (v) =>
        /^COM\d+$/i.test(v.trim()) ? null : "格式应为 COMx（如 COM3）",
    });
    if (!input) return;
    await updateConfig("port", input.trim().toUpperCase());
    statusBar.refresh();
    return;
  }
  if (picked.value) {
    await updateConfig("port", picked.value);
    statusBar.refresh();
  }
}

async function refreshPorts(): Promise<void> {
  const res = await listSerialPorts();
  if (res.error) {
    void vscode.window.showErrorMessage(`wsl-idf: 枚举 COM 口失败 - ${res.error}`);
    return;
  }
  void vscode.window.showInformationMessage(
    res.ports.length > 0
      ? `wsl-idf: 发现 ${res.ports.length} 个端口 - ${res.ports.join(", ")}`
      : "wsl-idf: 未发现 COM 端口",
  );
}

async function setBaud(): Promise<void> {
  const current = readConfig().monitorBaud;
  const presets = [
    "115200",
    "9600",
    "57600",
    "230400",
    "460800",
    "921600",
    "1500000",
  ];
  const picked = await vscode.window.showQuickPick(
    [
      ...presets.map((b) => ({
        label: b,
        description: Number(b) === current ? "（当前）" : undefined,
      })),
      { label: "$(edit) 自定义...", description: "手动输入波特率" },
    ],
    { title: "选择 Monitor 波特率", placeHolder: String(current) },
  );
  if (!picked) return;

  let baud: number;
  if (picked.label.startsWith("$(edit)")) {
    const v = await vscode.window.showInputBox({
      title: "输入波特率",
      value: String(current),
      validateInput: (s) =>
        /^\d+$/.test(s) && Number(s) > 0 ? null : "请输入正整数",
    });
    if (!v) return;
    baud = Number(v);
  } else {
    baud = Number(picked.label);
  }
  await updateConfig("monitorBaud", baud);
  statusBar.refresh();
}

async function toggleAutoMonitor(): Promise<void> {
  const next = !readConfig().autoMonitor;
  await updateConfig("autoMonitor", next);
  void vscode.window.showInformationMessage(
    `wsl-idf: 烧录后自动 Monitor ${next ? "已开启" : "已关闭"}`,
  );
}

async function showQuickMenu(): Promise<void> {
  const cfg = readConfig();
  type Item = vscode.QuickPickItem & { cmd?: string };

  const items: Item[] = [
    { label: "$(plug) 端口设置", kind: vscode.QuickPickItemKind.Separator },
    {
      label: "选择 / 切换 COM 端口",
      description: cfg.port || "未选择",
      cmd: "wsl-idf.selectPort",
    },
    { label: "刷新 COM 列表", cmd: "wsl-idf.refreshPorts" },
    {
      label: "设置 Monitor 波特率",
      description: String(cfg.monitorBaud),
      cmd: "wsl-idf.setBaud",
    },
    {
      label: `烧录后自动 Monitor: ${cfg.autoMonitor ? "ON" : "OFF"}`,
      cmd: "wsl-idf.toggleAutoMonitor",
    },

    { label: "$(tools) 构建", kind: vscode.QuickPickItemKind.Separator },
    { label: "idf.py build", cmd: "wsl-idf.build" },
    { label: "idf.py menuconfig", cmd: "wsl-idf.menuconfig" },
    { label: "idf.py clean", cmd: "wsl-idf.clean" },
    { label: "idf.py fullclean", cmd: "wsl-idf.fullclean" },

    { label: "$(zap) 烧录 / 擦除", kind: vscode.QuickPickItemKind.Separator },
    { label: "Flash (全部分区)", cmd: "wsl-idf.flash" },
    { label: "Flash App Only", cmd: "wsl-idf.flashApp" },
    { label: "Erase + Flash", cmd: "wsl-idf.flashErase" },
    { label: "Erase Flash", cmd: "wsl-idf.erase" },
    { label: "Merge Bin", cmd: "wsl-idf.merge" },
    { label: "Extract Bins (提取分区 bin)", cmd: "wsl-idf.extractBins" },

    { label: "$(terminal) 监视", kind: vscode.QuickPickItemKind.Separator },
    { label: "Open Monitor", cmd: "wsl-idf.monitor" },

    { label: "$(info) 其他", kind: vscode.QuickPickItemKind.Separator },
    { label: "About (WHEAT)", cmd: "wsl-idf.about" },
  ];

  const picked = await vscode.window.showQuickPick(items, {
    title: "wsl-idf 快捷菜单",
    placeHolder: `端口: ${cfg.port || "(未选择)"}   波特率: ${cfg.monitorBaud}`,
  });
  if (picked?.cmd) {
    void vscode.commands.executeCommand(picked.cmd);
  }
}
