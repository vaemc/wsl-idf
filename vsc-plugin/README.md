# wsl-idf（VS Code 扩展）

为 [wsl-idf](https://github.com/vaemc/wsl-idf) CLI 提供的 VS Code 扩展：在状态栏与命令面板中触发烧录、擦除、合并、提取 bin、串口监视，以及常用 `idf.py` 操作；命令发送到集成终端执行。

扩展本身不实现烧录逻辑，依赖本机可调用的 `wsl-idf` 与已 `source` 的 ESP-IDF 环境。CLI 说明见仓库根目录 [`README.MD`](../README.MD)。

---

## 目录

- [概述](#概述)
- [功能一览](#功能一览)
- [系统要求](#系统要求)
- [安装](#安装)
- [界面结构](#界面结构)
- [全局能力](#全局能力)
- [各功能模块](#各功能模块)
- [命令面板一览](#命令面板一览)
- [配置](#配置)
- [限制与注意](#限制与注意)
- [常见问题](#常见问题)
- [功能状态](#功能状态)
- [许可证](#许可证)

---

## 概述

| 模块 | 说明 |
|------|------|
| 状态栏 | 左侧一排按钮：端口、快捷菜单、Build / Flash / App / Erase+Flash / Erase / Merge / Extract / Monitor |
| 快捷菜单 | QuickPick 分组菜单，覆盖端口设置、构建、烧录、监视、About |
| 命令面板 | 全部 `wsl-idf:*` 命令 |
| 终端执行 | 将拼好的命令 `sendText` 到专用或活动终端 |
| 端口枚举 | 通过 `powershell.exe` 查询 `Win32_SerialPort` |

激活时机：`onStartupFinished`（窗口启动完成后显示状态栏）。

---

## 功能一览

| 能力 | 入口 | 终端等效命令（示意） |
|------|------|----------------------|
| 选择 COM | 状态栏端口按钮 / 快捷菜单 / 命令面板 | （写入配置 `wsl-idf.port`，不执行 CLI） |
| 刷新 COM 列表 | 命令面板 / 快捷菜单 | （信息提示，不执行 CLI） |
| 设置 Monitor 波特率 | 命令面板 / 快捷菜单 | （写入 `wsl-idf.monitorBaud`） |
| 切换烧录后自动 Monitor | 命令面板 / 快捷菜单 | （写入 `wsl-idf.autoMonitor`） |
| Build | 状态栏 `Build` | `idf.py build` |
| Flash | 状态栏 `Flash` | `wsl-idf -c flash -p <port> [monitor <baud>]` |
| Flash App | 状态栏 `App` | `wsl-idf -c flash-app -p <port> [monitor <baud>]` |
| Erase + Flash | 状态栏 `Erase+Flash` | `wsl-idf -c flash-erase -p <port> [monitor <baud>]` |
| Erase | 状态栏 `Erase` | `wsl-idf -c erase -p <port> [monitor <baud>]` |
| Merge | 状态栏 `Merge` | `wsl-idf -c merge` |
| Extract Bins | 状态栏 `Extract` | `wsl-idf -c extract-bins` |
| Monitor | 状态栏 `Monitor` | `wsl-idf -p <port> monitor <baud>` |
| menuconfig / clean / fullclean | 快捷菜单 / 命令面板 | `idf.py …` |
| About | 快捷菜单 / 命令面板 | `<executable> --about` |

说明：当 `wsl-idf.autoMonitor` 为 `true`（默认）时，`flash` / `flash-app` / `flash-erase` / `erase` 都会在命令末尾追加 `monitor <波特率>`。`merge` 与 `extract-bins` 不会追加。

---

## 系统要求

| 项 | 要求 |
|----|------|
| 编辑器 | VS Code / Cursor 等兼容环境，`engines.vscode` ≥ `^1.85.0` |
| 远程/终端 | 工作区在 WSL2 中打开，或默认终端 Profile 为 WSL，能执行 Linux 命令 |
| CLI | `wsl-idf` 已安装且在终端 PATH 中可调用（可用 `wsl-idf.executable` 指定路径） |
| ESP-IDF | 终端中已 `source` export（`idf.py` 可用） |
| 工作区 | 建议打开 **ESP-IDF 项目根目录** |
| Windows 侧 | 能从 WSL 调用 `powershell.exe`；烧录/监视依赖主项目所述 `ESPTOOL` 与 COM 驱动 |

---

## 安装

### 从 VSIX 安装

仓库或本地已有打包文件时（例如 `wsl-idf-0.1.0.vsix`）：

1. 扩展视图 → `…` → `Install from VSIX...`
2. 选择 `.vsix` 文件
3. 按需重载窗口

### 从源码打包

```bash
cd vsc-plugin
npm install
npm run compile
npx @vscode/vsce package
# 或：npm run package
# 生成 wsl-idf-0.1.0.vsix
```

### 开发调试

1. 用 VS Code 打开 `vsc-plugin` 目录
2. `npm install && npm run compile`
3. 按 `F5` 启动「扩展开发宿主」

当前扩展版本：`0.1.0`（以 `package.json` 为准）。

---

## 界面结构

扩展无独立 Webview 页，界面为：

1. **状态栏（左侧）**：一组可点击按钮  
2. **QuickPick**：端口选择、波特率、快捷菜单  
3. **集成终端**：实际执行命令的位置  

<!-- 截图占位：状态栏 -->
![状态栏](images/statusbar.png)

<!-- 截图占位：快捷菜单 -->
![快捷菜单](images/quick-menu.png)

### 状态栏按钮（完整模式）

从左到右（Codicon 图标 + 文案，以实际 UI 为准）：

| 显示文本 | 命令 | 说明 |
|----------|------|------|
| `选择端口` 或已选 `COMx` | `wsl-idf.selectPort` | 未选中时文案为「选择端口」 |
| `wsl-idf` | `wsl-idf.showMenu` | 打开快捷菜单 |
| `Build` | `wsl-idf.build` | `idf.py build` |
| `Flash` | `wsl-idf.flash` | 完整烧录 |
| `App` | `wsl-idf.flashApp` | 仅烧录 app |
| `Erase+Flash` | `wsl-idf.flashErase` | 擦除后烧录 |
| `Erase` | `wsl-idf.erase` | 仅擦除 |
| `Merge` | `wsl-idf.merge` | 合并固件 |
| `Extract` | `wsl-idf.extractBins` | 提取分区 bin |
| `Monitor` | `wsl-idf.monitor` | 串口监视 |

紧凑模式（`wsl-idf.statusBar.compact = true`）仅保留：**端口**与 **`wsl-idf` 快捷菜单**。

---

## 全局能力

### 终端选择

| 配置 | 默认 | 行为 |
|------|------|------|
| `wsl-idf.reuseActiveTerminal` | `false` | 使用名为 `wsl-idf.terminalName`（默认 `wsl-idf`）的终端；不存在则创建 |
| 同上为 `true` | — | 使用当前活动终端 |

发送命令前会 `show` 该终端，再 `sendText` 整行命令。扩展不解析命令退出码。

### 端口未选择时

需要 COM 口的操作（烧录/擦除/监视）若 `wsl-idf.port` 为空，弹出警告：

> `wsl-idf: 尚未选择 COM 口。点击状态栏端口按钮选择。`

并提供按钮「立即选择」，点击后执行 `wsl-idf.selectPort`。

### 配置变更

修改任意 `wsl-idf.*` 配置后，状态栏会重建以反映紧凑模式、优先级等变化。

---

## 各功能模块

### 选择 / 切换 COM 端口

**用途**：选定 Windows COM 口，写入工作区配置（失败时回退到全局配置）。

**入口**：状态栏端口按钮；命令 `wsl-idf: 选择串口 (COM Port)`；快捷菜单「选择 / 切换 COM 端口」。

**前置条件**：WSL 可调用 `powershell.exe`；设备已连接更佳。

**操作流程**：

1. 显示进度「wsl-idf: 枚举 COM 口...」
2. QuickPick 标题：「选择 wsl-idf 使用的 Windows COM 端口」
3. 可选已枚举端口、手动输入、刷新列表
4. 手动输入校验：`COMx`（如 `COM3`），保存时转为大写

**限制**：枚举失败时仍可手动输入；列表为空时提示确认设备与驱动。

---

### 刷新 COM 列表

**用途**：重新枚举并弹出信息，不打开选择器。

**入口**：`wsl-idf: 刷新串口列表`；快捷菜单「刷新 COM 列表」。

成功示例：`wsl-idf: 发现 N 个端口 - COM3, COM20`  
失败：错误提示含失败原因。

---

### 设置 Monitor 波特率

**用途**：设置尾部 `monitor` 使用的波特率。

**入口**：`wsl-idf: 设置 Monitor 波特率`；快捷菜单「设置 Monitor 波特率」。

**预设**：`115200`、`9600`、`57600`、`230400`、`460800`、`921600`、`1500000`，另可自定义正整数。

**默认值**：`115200`（配置项 `wsl-idf.monitorBaud`）。

---

### 烧录后自动 Monitor

**用途**：开关烧录/擦除类命令是否自动追加 `monitor <baud>`。

**入口**：`wsl-idf: 切换 烧录后自动 Monitor`；快捷菜单中 `烧录后自动 Monitor: ON|OFF`。

切换后提示：`wsl-idf: 烧录后自动 Monitor 已开启` / `已关闭`。

**默认**：`true`。

**影响范围**：`Flash` / `App` / `Erase+Flash` / `Erase`。不影响 `Merge`、`Extract`、单独 `Monitor`。

---

### Build / menuconfig / clean / fullclean

**用途**：在终端直接运行对应 `idf.py` 子命令（不经过 `wsl-idf` CLI）。

| UI / 命令 | 发送到终端 |
|-----------|------------|
| 状态栏 `Build` / `wsl-idf: idf.py build` | `idf.py build` |
| `wsl-idf: idf.py menuconfig` | `idf.py menuconfig` |
| `wsl-idf: idf.py clean` | `idf.py clean` |
| `wsl-idf: idf.py fullclean` | `idf.py fullclean` |

**前置条件**：终端内 ESP-IDF 环境已就绪；工作区为项目根更稳妥。

**限制**：`fullclean` / `clean` 会清理构建产物，属破坏性工程操作，确认后再点。

---

### Flash / App / Erase+Flash / Erase

**用途**：调用 CLI 烧录或擦除；是否追加 monitor 见 `autoMonitor`。

| 状态栏 | CLI 子命令 | 需要端口 | 破坏性 |
|--------|------------|----------|--------|
| `Flash` | `flash` | 是 | 改写分区 |
| `App` | `flash-app` | 是 | 改写 app |
| `Erase+Flash` | `flash-erase` | 是 | **整片擦除后烧录** |
| `Erase` | `erase` | 是 | **整片擦除** |

**示例**（端口 `COM3`，波特率 `115200`，`autoMonitor=true`）：

```text
wsl-idf -c flash -p COM3 monitor 115200
```

`autoMonitor=false` 时无尾部 `monitor …`。

**前置条件**：`wsl-idf` 与 `ESPTOOL` 等按主项目 README 配置；已选 COM。

---

### Merge / Extract

| 状态栏 | 命令 | 需要端口 | 终端命令 |
|--------|------|----------|----------|
| `Merge` | `wsl-idf.merge` | 否 | `wsl-idf -c merge` |
| `Extract` | `wsl-idf.extractBins` | 否 | `wsl-idf -c extract-bins` |

输出文件规则与风险见主项目 README（`full-<目录名>.bin`；`flash-bins/` 每次重建）。

---

### Monitor

**用途**：仅打开串口监视。

**入口**：状态栏 `Monitor`；快捷菜单「Open Monitor」；`wsl-idf: Monitor (串口监视)`。

**终端命令**：`wsl-idf -p <port> monitor <baud>`（baud 来自配置）。

**前置条件**：已选 COM。退出监视在终端按 Ctrl+C（由 CLI/PowerShell 侧处理）。

---

### 快捷菜单

**用途**：单入口覆盖端口、构建、烧录、监视、About。

**入口**：状态栏 `wsl-idf`；命令 `wsl-idf: 显示快捷菜单`。

**菜单标题**：`wsl-idf 快捷菜单`  
**占位提示**：`端口: …   波特率: …`

分组（分隔标题以 UI 为准）：

1. 端口设置：选择/切换、刷新、波特率、自动 Monitor  
2. 构建：build / menuconfig / clean / fullclean  
3. 烧录 / 擦除：Flash、Flash App Only、Erase + Flash、Erase Flash、Merge Bin、Extract Bins  
4. 监视：Open Monitor  
5. 其他：About (WHEAT)

---

### About

**用途**：在终端执行 `<wsl-idf.executable> --about`，显示 CLI 作者与仓库信息。

**入口**：快捷菜单「About (WHEAT)」；`wsl-idf: 显示 About (WHEAT)`。

---

## 命令面板一览

`Ctrl+Shift+P`（macOS：`Cmd+Shift+P`）输入 `wsl-idf:`：

| 命令标题 |
|----------|
| `wsl-idf: 选择串口 (COM Port)` |
| `wsl-idf: 刷新串口列表` |
| `wsl-idf: 设置 Monitor 波特率` |
| `wsl-idf: 切换 烧录后自动 Monitor` |
| `wsl-idf: idf.py build` |
| `wsl-idf: Flash (烧录)` |
| `wsl-idf: Flash App Only (仅烧录 app)` |
| `wsl-idf: Erase + Flash (擦除后烧录)` |
| `wsl-idf: Erase Flash (擦除)` |
| `wsl-idf: Merge Bin (合并固件)` |
| `wsl-idf: Extract Bins (提取分区 bin)` |
| `wsl-idf: Monitor (串口监视)` |
| `wsl-idf: idf.py menuconfig` |
| `wsl-idf: idf.py clean` |
| `wsl-idf: idf.py fullclean` |
| `wsl-idf: 显示 About (WHEAT)` |
| `wsl-idf: 显示快捷菜单` |

可在键盘快捷方式中为上述命令绑定按键。

---

## 配置

设置中搜索 `wsl-idf`：

| 配置项 | 类型 | 默认值 | 含义 |
|--------|------|--------|------|
| `wsl-idf.executable` | string | `wsl-idf` | CLI 可执行文件路径或命令名（须在终端可调用） |
| `wsl-idf.port` | string | `""` | 当前 Windows COM 口；选择端口后写入 |
| `wsl-idf.monitorBaud` | number | `115200` | Monitor 波特率 |
| `wsl-idf.autoMonitor` | boolean | `true` | 烧录/擦除类命令后自动追加 `monitor` |
| `wsl-idf.terminalName` | string | `wsl-idf` | 专用终端名称 |
| `wsl-idf.reuseActiveTerminal` | boolean | `false` | `true` 时用活动终端，否则用专用终端 |
| `wsl-idf.statusBar.priority` | number | `100` | 状态栏起始优先级（越大越靠左） |
| `wsl-idf.statusBar.compact` | boolean | `false` | 紧凑模式：仅端口 + 快捷菜单 |

端口与波特率等更新默认写入 **Workspace**；工作区不可写时回退到 **Global**。

本扩展不另建数据目录；无独立落盘文件（配置由 VS Code settings 管理）。

---

## 限制与注意

1. 扩展只负责拼命令并送入终端，不保证 CLI/IDF 执行成功。  
2. 终端必须是 WSL（或能跑 `wsl-idf` / `idf.py` 的环境）；纯 Windows PowerShell/CMD Profile 通常不可用。  
3. `Erase` / `Erase+Flash` 为破坏性操作；`idf.py fullclean` / `clean` 会清理构建目录。  
4. `autoMonitor` 开启时，`Erase` 也会追加 monitor（与代码一致）。  
5. 未发布到 Marketplace 时，需自行用 VSIX 安装。  
6. 状态栏图标为 VS Code Codicon（如 plug / rocket），不是 Markdown emoji。

---

## 常见问题

### 状态栏未显示

- 命令面板执行 `Developer: Reload Window`
- 确认扩展已启用
- 输出面板查看激活错误

### 点击后终端无反应或命令找不到

- 在同一终端手动执行 `wsl-idf --version`、`idf.py --version`
- 检查默认终端 Profile 是否为 WSL
- 调整 `wsl-idf.executable` 为绝对路径
- 需要复用已 `source` 的 IDF 终端时，打开 `wsl-idf.reuseActiveTerminal`

### COM 列表为空或枚举失败

- 设备管理器中确认 COM
- WSL 自测：  
  `powershell.exe -Command "Get-WmiObject Win32_SerialPort | Select-Object -ExpandProperty DeviceID"`
- 使用「手动输入...」填写 `COMx`

### 未选择端口时点击 Flash / Monitor

见上文警告与「立即选择」。

---

## 功能状态

| 功能 | 状态 |
|------|------|
| 状态栏完整 / 紧凑模式 | 已实现 |
| COM 枚举、手动输入、刷新 | 已实现 |
| 波特率预设与自定义 | 已实现 |
| autoMonitor 开关 | 已实现 |
| CLI：flash / flash-app / flash-erase / erase / merge / extract-bins / monitor / --about | 已实现 |
| idf.py：build / menuconfig / clean / fullclean | 已实现 |
| 快捷菜单与命令面板 | 已实现 |
| Marketplace 在线安装 | 未实现（当前以 VSIX / 源码为主） |
| 扩展内嵌串口视图（不经终端） | 未实现 |
| 命令执行结果解析 / 状态回传 | 未实现 |

---

## 许可证

MIT © wheat \<vaemc520@qq.com\>
