# wsl-idf VSCode 插件

为 [wsl-idf](https://github.com/vaemc/wsl-idf) CLI 提供 VSCode 状态栏快捷操作：一键 Build、Flash、Monitor，Windows COM 口自动枚举与下拉选择，命令直接发送到 VSCode 集成终端。

> 适合在 **WSL2 + ESP-IDF** 中开发，又希望通过 Windows 侧 `esptool.exe` 烧录和监视的开发者。

---

## 功能

- **底部状态栏**：端口选择按钮 + Build / Flash / FlashApp / Erase+Flash / Erase / Merge / Monitor
- **端口下拉**：调用 `powershell.exe` 枚举 Windows 上的 `COMx`，支持手动输入与刷新
- **一键命令**：点击按钮自动在终端执行 `wsl-idf -c flash -p COM3`，自动追加端口
- **烧录后自动 Monitor**：可在配置中开关，开启后等效 `wsl-idf -c flash -p COM3 monitor 115200`
- **快捷菜单**：点击状态栏 `🚀 wsl-idf` 进入分组式 QuickPick，覆盖所有功能
- **覆盖 wsl-idf 全部子命令**：`flash` / `flash-app` / `flash-erase` / `erase` / `merge` / `monitor` / `--about`
- **额外 IDF 操作**：`idf.py build` / `menuconfig` / `clean` / `fullclean`

---

## 先决条件

1. WSL2 + ESP-IDF 环境，已安装并 `source export.sh`
2. 已编译并安装 `wsl-idf`（命令可在 WSL shell 直接调用，参考主项目 README）
3. VSCode 当前打开的工作区位于 **ESP-IDF 项目根目录**
4. 终端默认 shell 是 WSL（VSCode → Terminal → Default Profile 选 WSL）

---

## 安装

### 方式 A：从源码打包

```bash
cd vsc-plugin
npm install
npm run compile
npx @vscode/vsce package
# 生成 wsl-idf-0.1.0.vsix
```

随后在 VSCode：`扩展` → `…` → `Install from VSIX...` → 选择上面生成的 `.vsix`。

### 方式 B：开发调试

1. 在 VSCode 中打开 `vsc-plugin` 文件夹
2. `npm install && npm run compile`
3. 按 `F5` 启动「扩展开发宿主」窗口

---

## 使用

### 1. 选择 COM 端口

- 点击状态栏左下角 `🔌 选择端口`
- 插件会调用 `powershell.exe` 列出所有可见 `COMx`
- 也可手动输入 / 刷新

### 2. 烧录

- 点击 `⚡ Flash` → 终端运行 `wsl-idf -c flash -p COM3`（端口取自配置）
- 若开启了「烧录后自动 Monitor」，将自动追加 `monitor 115200`

### 3. 监视

- 单独 Monitor：点击 `🖥 Monitor`，等效 `wsl-idf -p COM3 monitor 115200`

### 4. 其他

| 状态栏按钮 | 等效命令 |
|------------|----------|
| `🛠 Build` | `idf.py build` |
| `⚡ Flash` | `wsl-idf -c flash -p <port> [monitor <baud>]` |
| `🚀 App`   | `wsl-idf -c flash-app -p <port> [monitor <baud>]` |
| `🔥 Erase+Flash` | `wsl-idf -c flash-erase -p <port> [monitor <baud>]` |
| `🗑 Erase` | `wsl-idf -c erase -p <port>` |
| `📦 Merge` | `wsl-idf -c merge` |
| `🖥 Monitor` | `wsl-idf -p <port> monitor <baud>` |
| `🚀 wsl-idf` | 弹出分组快捷菜单（含 menuconfig / clean / About 等） |

---

## 命令面板

`Ctrl+Shift+P` 输入 `wsl-idf:` 可看到全部命令：

- `wsl-idf: 选择串口 (COM Port)`
- `wsl-idf: 刷新串口列表`
- `wsl-idf: 设置 Monitor 波特率`
- `wsl-idf: 切换 烧录后自动 Monitor`
- `wsl-idf: idf.py build / menuconfig / clean / fullclean`
- `wsl-idf: Flash / Flash App Only / Erase + Flash / Erase / Merge / Monitor`
- `wsl-idf: 显示 About (WHEAT)`
- `wsl-idf: 显示快捷菜单`

可通过 `Preferences: Open Keyboard Shortcuts` 给常用命令绑定快捷键。

---

## 配置

`Ctrl+,` 搜索 `wsl-idf`：

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `wsl-idf.executable` | `wsl-idf` | CLI 可执行文件路径或命令名 |
| `wsl-idf.port` | `""` | 当前 COM 口（状态栏选择后自动写入） |
| `wsl-idf.monitorBaud` | `115200` | Monitor 波特率 |
| `wsl-idf.autoMonitor` | `true` | 烧录类命令后自动追加 `monitor` |
| `wsl-idf.terminalName` | `wsl-idf` | 专用终端名称（不存在则自动创建） |
| `wsl-idf.reuseActiveTerminal` | `false` | 直接在当前活动终端执行，而非专用终端 |
| `wsl-idf.statusBar.priority` | `100` | 状态栏起始优先级（越大越靠左） |
| `wsl-idf.statusBar.compact` | `false` | 紧凑模式：仅保留端口与快捷菜单按钮 |

---

## 常见问题

### 状态栏没显示

- 重新加载窗口：命令面板 → `Developer: Reload Window`
- 检查 `wsl-idf.statusBar.compact` 是否被开启
- 输出面板 → 选择 `Window` 频道查看激活错误

### 点击按钮终端没反应

- 终端必须能调用 `wsl-idf`（先在终端内手动测试）
- 默认 shell 需为 WSL；若使用 PowerShell/CMD 则无法执行 Linux 命令
- 若想复用已有 IDF 终端，开启 `wsl-idf.reuseActiveTerminal`

### COM 端口列表为空

- 确认设备已连接、Windows 设备管理器中可见
- 在 WSL 终端执行：`powershell.exe -Command "Get-WmiObject Win32_SerialPort | Select-Object DeviceID"`
- 检查 `powershell.exe` 是否在 WSL `PATH` 中

### 未选择端口时点击 Flash

会弹出提示「尚未选择 COM 口」并提供「立即选择」按钮，点击后即进入端口选择。

---

## 许可证

MIT © wheat <vaemc520@qq.com>
