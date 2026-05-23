# wsl-idf

**中文** | [English](./README.en.md)

wsl-idf 是面向 WSL2 + ESP-IDF 开发的辅助工具。
你在 WSL 里用 idf.py 写代码、编译；USB 串口在 Windows 上时，用它自动调用 Windows 侧的 esptool.exe 完成烧录、擦除、固件合并和串口监视。
支持完整烧录、仅刷 app、全片擦除后烧录；串口监视带 ESP-IDF 日志着色，崩溃时还能自动用 addr2line 解析地址。

---

## 目录

- [背景与适用场景](#背景与适用场景)
- [功能特性](#功能特性)
- [系统要求](#系统要求)
- [安装与编译](#安装与编译)
- [配置](#配置)
- [快速开始](#快速开始)
- [命令参考](#命令参考)
- [串口监视](#串口监视)
- [崩溃地址自动解析](#崩溃地址自动解析)
- [工作原理](#工作原理)
- [典型工作流](#典型工作流)
- [常见问题](#常见问题)

---

## 背景与适用场景

ESP-IDF 官方工具链运行在 Linux/WSL 中非常顺畅，但 USB 转串口设备通常挂载在 **Windows 主机** 上，WSL 无法直接访问 `COM` 口。常见做法是：

- 在 Windows 里单独装一套 ESP-IDF / esptool；或
- 手动切换终端、复制路径、拼接 esptool 参数。

**wsl-idf** 解决的是：在 WSL 里用 `idf.py` 编译，烧录和监视仍走 Windows 的 `esptool.exe` 与 COM 口，且命令行体验尽量贴近 `idf.py flash` / `idf.py monitor`。

适合以下情况：

| 场景 | 说明 |
|------|------|
| WSL2 + ESP-IDF 开发 | 代码、编译在 WSL；硬件 USB 在 Windows |
| 已有 Windows 版 esptool | 无需在 WSL 再配置 USB 串口转发 |
| 日常烧录 + 调试 | 一条命令完成 flash / 仅刷 app / 擦除 / 合并 / 监视 |

---

## 功能特性

### 固件操作

| 功能 | 说明 |
|------|------|
| **flash** | 先执行 `idf.py build`，再按 `build/flasher_args.json` 烧录全部分区 |
| **flash-app** | 先 `idf.py build`，再仅烧录 `app` 分区（适合频繁改应用逻辑、不改分区表/Bootloader 时） |
| **flash-erase** | 先 `idf.py build`，再 `--erase-all` 全片擦除后完整烧录 |
| **erase** | 仅擦除 Flash，不编译、不烧录 |
| **merge** | 将各分区 bin 合并为单个 `full-<项目名>.bin`（无需 COM 口） |

### 串口监视

- 通过 Windows PowerShell 打开 COM 口，UTF-8 输出
- **ESP-IDF 日志自动着色**（E/W/I/D/V 等级，配色接近 `idf.py monitor`）
- 支持 `@@@` 前缀自定义高亮（Info 级日志显示为紫色并去掉标记）
- **崩溃检测 + addr2line 自动解析**（Guru Meditation、看门狗、assert 等）

### 其他

- 每次启动时列出 Windows 上可用 **COM 口**，便于确认 `-p` 参数
- 自动读取芯片型号、烧录偏移与文件路径，无需手写 esptool 参数
- WSL 路径经 `wslpath -w` 转为 Windows 路径后交给 esptool

---

## 系统要求

### WSL 侧

- WSL2（推荐 Ubuntu 等）
- 已安装并 `source` 的 **ESP-IDF** 环境（含 `idf.py`）
- **Rust 工具链**（用于编译本工具；使用预编译二进制则不需要）
- 当前工作目录为 **ESP-IDF 项目根目录**（含 `CMakeLists.txt` / `sdkconfig` 等）

### Windows 侧

- **esptool** 可执行文件（例如 `esptool.exe` 或官方发布的 `esptool-windows-amd64` 包）
- **PowerShell**（WSL 通过 `powershell.exe` 调用）
- ESP 开发板 USB 驱动已安装，设备管理器中可见 COM 口

### 崩溃解析（可选）

- 项目已成功 `idf.py build`，存在 `build/*.elf`
- WSL 中可找到对应架构的 `*-addr2line`（通常 `source $IDF_PATH/export.sh` 后即可）

---

## 安装与编译

```bash
git clone <本仓库地址> ~/rust-project/wsl-idf
cd ~/rust-project/wsl-idf
cargo build --release
```

编译产物：`target/release/wsl-idf`

建议加入 shell 配置（`~/.bashrc` 或 `~/.zshrc`）：

```bash
alias wsl-idf='$HOME/rust-project/wsl-idf/target/release/wsl-idf'
```

验证：

```bash
wsl-idf --help
wsl-idf --version
```

---

## 配置

### 1. ESPTOOL 环境变量（必需）

指向 **Windows 上** 的 esptool 可执行文件路径。在 WSL 中使用 Windows 风格路径：

```bash
export ESPTOOL='D:\\esptool-windows-amd64\\esptool.exe'
```

说明：

- 路径必须是 Windows 能访问的位置（`C:\`、`D:\` 等）
- 反斜杠在 shell 中需写成 `\\` 或使用合适引号
- 可将上述 `export` 写入 `~/.bashrc` / `~/.zshrc` 持久化

未设置时运行会报错：`未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）`

### 2. ESP-IDF 环境（WSL）

每次打开新终端开发 ESP 项目前：

```bash
source ~/esp/esp-idf/export.sh   # 按你的实际安装路径调整
```

烧录前需能正常执行 `idf.py build`；崩溃解析需能用到 `riscv32-esp-elf-addr2line` 或 `xtensa-*-addr2line` 等工具。

### 3. 确认 COM 口

连接开发板后，在 Windows **设备管理器 → 端口 (COM 和 LPT)** 中查看编号，例如 `COM3`、`COM20`。

运行任意 `wsl-idf` 命令时，会先打印当前 Windows 可见的串口列表（来自 `Win32_SerialPort`），例如：

```
COM1
COM3
COM20
```

---

## 快速开始

```bash
# 1. 进入 ESP-IDF 项目根目录
cd ~/projects/my-esp-project

# 2. 加载 IDF 环境
source ~/esp/esp-idf/export.sh

# 3. 确认 ESPTOOL 已配置
echo $ESPTOOL

# 4. 编译并烧录（内部会先 idf.py build）
wsl-idf -c flash -p COM3

# 5. 烧录完成后打开串口监视（默认 115200）
wsl-idf -c flash -p COM3 monitor

# 6. 仅监视（不烧录）
wsl-idf -p COM3 monitor
```

---

## 命令参考

### 通用选项

```
Usage: wsl-idf [OPTIONS] [TRAILING]...

Options:
  -p, --port <PORT>       设备端口号（Windows COM 口，如 COM3）
  -c, --command <COMMAND>  烧录相关子命令（见下表）
  -h, --help              帮助
  -V, --version           版本号

Arguments:
  [TRAILING]...           命令完成后串口监视：monitor [波特率]
```

### 子命令 `-c / --command`

| 子命令 | 是否需要 `-p` | 是否自动 `idf.py build` | 作用 |
|--------|---------------|-------------------------|------|
| `flash` | 是 | 是 | 按 `flasher_args.json` 烧录 bootloader、分区表、app 等全部分区 |
| `flash-app` | 是 | 是 | 仅烧录 app 分区 |
| `flash-erase` | 是 | 是 | 全片擦除后完整烧录 |
| `erase` | 是 | 否 | 仅执行 `erase-flash` |
| `merge` | 否 | 否 | 合并 bin 到项目目录下的 `full-<项目名>.bin` |

### 烧录类命令示例

```bash
# 完整烧录
wsl-idf -c flash -p COM3

# 仅烧录应用程序（开发迭代更快）
wsl-idf -c flash-app -p COM3

# 擦除整个 Flash 后重新烧录（换分区表、清 NVS 等场景）
wsl-idf -c flash-erase -p COM3

# 只擦除，不烧录
wsl-idf -c erase -p COM3

# 合并固件（生成单文件，便于量产或 OTA 包准备）
wsl-idf -c merge
# 输出示例：~/projects/my-esp-project/full-my-esp-project.bin
```

### 烧录参数说明（工具内部固定）

对 `flash` / `flash-app` / `flash-erase`，wsl-idf 会组装类似如下的 esptool 调用（具体分区与偏移来自 `build/flasher_args.json`）：

- 波特率：`-b 1152000`
- 复位：`--before default-reset --after hard-reset`
- Flash 模式：`--flash-mode dio`
- 芯片型号：从 `flasher_args.json` 的 `extra_esptool_args.chip` 读取

你一般 **不需要** 手动指定这些参数。

### 仅列出串口 / 仅监视

```bash
# 只打印 COM 口列表（未指定 -c 时除列表外无其他操作）
wsl-idf

# 仅打开串口监视，默认 115200
wsl-idf -p COM3 monitor

# 指定波特率
wsl-idf -p COM3 monitor 9600
```

### 烧录 + 监视组合

在命令末尾追加 `monitor` 或 `monitor <波特率>`，**先完成 `-c` 指定操作，再进入监视**。使用 `monitor` 时必须提供 `-p`。

| 场景 | 命令 |
|------|------|
| 烧录后监视 | `wsl-idf -c flash -p COM3 monitor` |
| 烧录后监视（9600） | `wsl-idf -c flash -p COM3 monitor 9600` |
| 仅烧录 app 后监视 | `wsl-idf -c flash-app -p COM3 monitor 115200` |
| 擦除后烧录并监视 | `wsl-idf -c flash-erase -p COM3 monitor` |
| 擦除后监视 | `wsl-idf -c erase -p COM3 monitor 115200` |
| 仅监视 | `wsl-idf -p COM3 monitor` |

监视过程中按 **Ctrl+C** 退出；PowerShell 脚本会关闭串口。

---

## 串口监视

### 日志着色

监视输出会识别 ESP-IDF 标准日志格式，例如：

```
I (637787) wifi: station connected
E (640797) main: [ERROR] 连接失败
W (643817) heap: 内存使用率偏高
```

等级与颜色（ANSI）：

| 等级 | 字符 | 颜色 |
|------|------|------|
| Error | E | 红 |
| Warn | W | 黄 |
| Info | I | 绿 |
| Debug | D | 青 |
| Verbose | V | 灰 |

### 自定义高亮：`@@@` 标记

若 Info 级日志正文以 `@@@` 开头，会以 **紫色** 显示，并自动去掉 `@@@` 前缀，便于在大量日志中突出关键信息：

```
# 设备打印
I (1000) LOG_DEMO: @@@自定义高亮

# 终端显示（紫色，且无 @@@）
I (1000) LOG_DEMO: 自定义高亮
```

在固件中可这样写：

```c
ESP_LOGI(TAG, "@@@%s", "自定义高亮");
```

---

## 崩溃地址自动解析

串口监视运行时，wsl-idf 会监听 ESP 崩溃相关日志。检测到崩溃块结束后，自动收集其中的 **十六进制地址**，调用 WSL 中的 **addr2line** 解析符号，并在监视输出中插入：

```
--- wsl-idf addr2line ---
<函数名、文件名、行号等>
```

### 触发条件（崩溃开始）

日志行包含以下任一内容时进入「崩溃收集」状态：

- `Guru Meditation Error`
- `register dump`
- `Backtrace:`
- `abort() was called`
- `Stack memory:`
- `Task watchdog got triggered`
- `assert failed`
- `Panic handler`

### 结束条件

- `Please enable CONFIG_ESP_SYSTEM_USE_FRAME_POINTER`
- `ELF file SHA256`
- `Rebooting...`
- `Backtrace stopped`

### 地址来源

- 寄存器 dump、异常信息行中的 `0x........` 地址
- `Backtrace:` 行中 `PC:SP` 对的 **PC** 部分
- 自动忽略占位地址：`0x00000000`、`0xdeadc0de`、`0xdeadbeef` 等

### 启用前提

1. 在 **ESP-IDF 项目根目录** 下运行监视（需读取 `build/`）
2. 存在 ELF：优先 `build/project_description.json` 的 `app_elf`，否则 `build/<项目名>.elf`
3. 能找到 addr2line：优先 `build/CMakeCache.txt` 中的 `CMAKE_ADDR2LINE`，否则在 `PATH` 中查找 `riscv32-esp-elf-addr2line`、`xtensa-esp32-elf-addr2line` 等

若条件不满足，启动监视时会提示（不影响正常看日志）：

```
wsl-idf: 崩溃地址自动解析未启用（未找到 build/*.elf）
```

### 建议

- 崩溃前确保 **`idf.py build` 与当前运行固件一致**，否则行号可能对不上
- 需要完整回溯时，可在 `sdkconfig` 中启用 `CONFIG_ESP_SYSTEM_USE_FRAME_POINTER`

---

## 工作原理

```
┌─────────────────────────────────────────────────────────────┐
│                        WSL2                                 │
│  ┌──────────────┐    ┌─────────────────┐    ┌──────────────┐  │
│  │   wsl-idf    │───▶│  idf.py build   │    │  addr2line   │  │
│  │   (Rust)     │    │  (可选/烧录前)   │    │  (崩溃解析)  │  │
│  └──────┬───────┘    └─────────────────┘    └──────────────┘  │
│         │ 读取 build/flasher_args.json                        │
│         │ wslpath -w 转换项目路径                               │
│         ▼                                                     │
│  ┌──────────────┐                                             │
│  │ powershell.exe│                                            │
│  └──────┬───────┘                                             │
└─────────┼─────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                      Windows                                │
│  ┌──────────────┐         ┌──────────────┐                  │
│  │  esptool.exe │────────▶│  COMx USB    │──▶ ESP 芯片      │
│  └──────────────┘         └──────────────┘                  │
│  ┌──────────────┐                                            │
│  │ SerialPort   │◀── monitor 读串口 ──▶ 回传 WSL 着色输出     │
│  └──────────────┘                                            │
└─────────────────────────────────────────────────────────────┘
```

要点：

1. **构建信息**：烧录列表来自 ESP-IDF 生成的 `build/flasher_args.json`（`flash_files`、`app`、`extra_esptool_args.chip`）。
2. **路径**：WSL 项目路径通过 `wslpath -w` 转为 `D:\...` 等形式，供 Windows 上的 esptool 读取 `build\` 下 bin 文件。
3. **监视**：PowerShell `System.IO.Ports.SerialPort` 读 COM 口；wsl-idf 对 stdout 做 ESP-IDF 着色与崩溃解析。

---

## 典型工作流

### 日常开发（改应用代码）

```bash
cd ~/projects/my-esp-project
source ~/esp/esp-idf/export.sh

# 改代码 → 仅刷 app + 监视
wsl-idf -c flash-app -p COM3 monitor
```

### 修改分区表 / Bootloader / 首次烧录

```bash
wsl-idf -c flash -p COM3 monitor
# 或需要清空整片 Flash：
wsl-idf -c flash-erase -p COM3 monitor
```

### 导出合并固件

```bash
# 需已有 build/flasher_args.json（至少 build 过一次）
wsl-idf -c merge
ls -la full-*.bin
```

### 排查崩溃

```bash
# 确保刚 build 过且 ELF 与设备内固件一致
wsl-idf -p COM3 monitor
# 触发崩溃后，终端会自动追加 addr2line 结果
```

---

## 常见问题

### 未设置 ESPTOOL

```
未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）
```

解决：按 [配置](#1-esptool-环境变量必需) 设置 `ESPTOOL` 为 Windows 上 esptool 的完整路径。

### 找不到 flasher_args.json

```
No such file or directory ... build/flasher_args.json
```

解决：在项目根目录执行 `idf.py build`，或直接使用 `wsl-idf -c flash`（会自动 build）。

### flash-app 报错缺少 app 字段

```
flasher_args.json 中缺少 app 字段
```

解决：确认 ESP-IDF 版本与项目配置正常；完整 `idf.py build` 后再试。部分特殊构建可能未生成 app 条目，可改用 `flash`。

### 串口打开失败

PowerShell 提示 `串口打开失败`：

- 确认 `-p` 与设备管理器中的 COM 号一致（区分大小写一般无影响，建议与 listed 输出一致）
- 关闭占用该 COM 口的其他程序（Arduino IDE、另一个 monitor、串口助手等）
- 重新插拔 USB 或更换数据线

### wslpath 失败

```
wslpath 失败（退出码 ...）
```

解决：确认在 WSL 内运行，且当前目录路径合法；必要时 `cd` 到项目目录再执行。

### 崩溃解析未启用或行号不对

- 在项目根目录运行，且 `build/*.elf` 存在
- 执行 `source export.sh` 保证 addr2line 在 PATH 中
- 重新烧录与 ELF 同一次 build 的固件后再复现崩溃

### COM 列表为空或缺少目标口

- 检查 Windows 驱动与 USB 连接
- WSL 需能调用 `powershell.exe`；在 WSL 终端执行 `powershell.exe -Command "Get-WmiObject Win32_SerialPort"` 自测

### merge 输出位置

合并文件写在 **当前项目根目录**：`full-<目录名>.bin`，例如项目文件夹为 `my-esp-project` 则生成 `full-my-esp-project.bin`。

---

## 与 idf.py 的对照

| 目的 | idf.py（原生 Linux USB） | wsl-idf（WSL + Windows COM） |
|------|--------------------------|------------------------------|
| 编译 | `idf.py build` | `flash*` 子命令内会自动 build；`merge`/`erase` 不会 |
| 烧录 | `idf.py flash` | `wsl-idf -c flash -p COMx` |
| 仅 app | `idf.py app-flash` | `wsl-idf -c flash-app -p COMx` |
| 擦除烧录 | `idf.py erase-flash flash` | `wsl-idf -c flash-erase -p COMx` |
| 监视 | `idf.py monitor` | `wsl-idf -p COMx monitor [波特率]` |
| 合并 bin | 需自行拼 esptool | `wsl-idf -c merge` |

wsl-idf **不能替代** 完整的 ESP-IDF 项目管理（menuconfig、clean、组件依赖等），这些仍应在 WSL 中用 `idf.py` 完成。

---

## 许可证

请参阅仓库中的 LICENSE 文件（如有）。本项目为 ESP-IDF 开发辅助工具，使用时请同时遵守 Espressif 与 esptool 的相关许可与文档。

---

## 相关链接

- [ESP-IDF 编程指南](https://docs.espressif.com/projects/esp-idf/zh_CN/latest/esp32/)
- [esptool 文档](https://docs.espressif.com/projects/esptool/en/latest/)
- [WSL 文档](https://learn.microsoft.com/zh-cn/windows/wsl/)
