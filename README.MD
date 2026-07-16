# wsl-idf

**中文** | [English](./README.en.md)

在 WSL2 中开发 ESP-IDF 项目时，通过 Windows 侧 `esptool` 与 COM 口完成烧录、擦除、固件合并与串口监视的命令行工具（Rust / clap）。

仓库另含 VS Code 状态栏扩展，见 [`vsc-plugin/`](./vsc-plugin/README.md)；本文档仅描述 CLI。

---

## 目录

- [概述](#概述)
- [功能一览](#功能一览)
- [系统要求](#系统要求)
- [安装与运行](#安装与运行)
- [配置](#配置)
- [全局行为](#全局行为)
- [命令参考](#命令参考)
  - [通用选项](#通用选项)
  - [flash](#flash)
  - [flash-app](#flash-app)
  - [flash-erase](#flash-erase)
  - [erase](#erase)
  - [merge](#merge)
  - [extract-bins](#extract-bins)
  - [串口监视（尾部参数）](#串口监视尾部参数)
- [串口监视：日志着色](#串口监视日志着色)
- [串口监视：崩溃地址解析](#串口监视崩溃地址解析)
- [数据与输出路径](#数据与输出路径)
- [工作原理](#工作原理)
- [与 idf.py 对照](#与-idfpy-对照)
- [常见问题](#常见问题)
- [功能状态](#功能状态)

---

## 概述

ESP-IDF 工具链可在 WSL 中编译工程，但 USB 串口通常挂在 Windows 主机上，WSL2 无法直接打开 `COMx`。`wsl-idf` 在 WSL 内调用 `idf.py build`（部分子命令），读取 `build/flasher_args.json`，经 `wslpath -w` 转换路径后，通过 `powershell.exe` 调用 Windows 上的 `esptool` 与串口。

模块划分：

| 模块 | 说明 |
|------|------|
| 固件操作 | `flash` / `flash-app` / `flash-erase` / `erase` / `merge` / `extract-bins` |
| 串口监视 | 尾部参数 `monitor [波特率]`；PowerShell 读 COM，本地着色与崩溃解析 |
| 辅助 | 启动时枚举 COM 口；`--about` 显示作者与仓库信息 |

工作目录必须是 **ESP-IDF 项目根目录**（存在 `build/` 产物或可执行 `idf.py build`）。

---

## 功能一览

| 能力 | 入口 | 说明 |
|------|------|------|
| 完整烧录 | `-c flash -p COMx` | 先 `idf.py build`，再按 `flasher_args.json` 烧录全部分区 |
| 仅烧录 app | `-c flash-app -p COMx` | 先 build，仅烧录 `app` 分区 |
| 擦除后烧录 | `-c flash-erase -p COMx` | 先 build，再 `write-flash ... --erase-all` |
| 仅擦除 | `-c erase -p COMx` | `erase-flash`，不编译、不烧录 |
| 合并 bin | `-c merge` | 合并为项目根目录 `full-<目录名>.bin`，无需端口 |
| 提取分区 bin | `-c extract-bins` | 复制到 `flash-bins/`，文件名带烧录偏移，无需端口与 ESPTOOL |
| 串口监视 | `-p COMx monitor [波特率]` | 可单独使用，或接在上述命令之后 |
| 列出 COM | （无 `-c`、无 monitor） | 仅打印 Windows 可见串口 |
| 关于 | `--about` | 打印作者、版权、仓库地址后退出 |

---

## 系统要求

### WSL 侧

| 项 | 要求 |
|----|------|
| 环境 | WSL2（推荐 Ubuntu 等） |
| ESP-IDF | 已安装，且当前 shell 已 `source` export（提供 `idf.py`） |
| 工作目录 | ESP-IDF 项目根目录 |
| 编译本工具 | Rust 工具链（使用已编译二进制时不需要） |
| 路径转换 | 系统提供 `wslpath` |
| 崩溃解析（可选） | 存在 `build/*.elf`，且可找到对应 `*-addr2line` |

### Windows 侧

| 项 | 要求 |
|----|------|
| esptool | 可执行文件（如 `esptool.exe`）；路径写入环境变量 `ESPTOOL` |
| PowerShell | WSL 可调用 `powershell.exe` |
| 驱动 | 开发板 USB 驱动已装，设备管理器中可见 COM 口 |

烧录类命令（`flash` / `flash-app` / `flash-erase` / `erase` / `merge`）依赖 `ESPTOOL`。`extract-bins` 与仅监视不依赖。

---

## 安装与运行

从源码构建：

```bash
git clone https://github.com/vaemc/wsl-idf.git
cd wsl-idf
cargo build --release
```

产物路径：`target/release/wsl-idf`。

可选：在 `~/.bashrc` 或 `~/.zshrc` 中设置别名：

```bash
alias wsl-idf='$HOME/path/to/wsl-idf/target/release/wsl-idf'
```

验证：

```bash
wsl-idf --help
wsl-idf --version
```

当前版本（以二进制为准）：`0.1.0`。

---

## 配置

### 环境变量 `ESPTOOL`（烧录 / 擦除 / 合并必需）

值为 **Windows 侧** esptool 的完整路径，在 WSL shell 中使用 Windows 路径写法：

```bash
export ESPTOOL='D:\\esptool-windows-amd64\\esptool.exe'
```

| 项 | 说明 |
|----|------|
| 路径位置 | 必须是 Windows 可访问路径（如 `C:\`、`D:\`） |
| 反斜杠 | shell 中写成 `\\`，或使用合适引号 |
| 持久化 | 写入 `~/.bashrc` / `~/.zshrc` |
| 未设置时报错 | `未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）` |

### ESP-IDF 环境

```bash
source ~/esp/esp-idf/export.sh   # 按本机安装路径调整
```

### 确认 COM 口

1. Windows：设备管理器 → 端口 (COM 和 LPT)
2. 或在项目目录执行 `wsl-idf`（无子命令），打印 `Win32_SerialPort` 的 `DeviceID` 列表

---

## 全局行为

1. 除 `--about` 外，每次启动都会先枚举并打印 Windows COM 口列表。
2. `--about` 仅打印关于信息后退出，不枚举串口、不执行其他操作。
3. 尾部可跟 `monitor` 或 `monitor <波特率>`：先执行 `-c` 指定操作（若有），再进入监视。
4. 使用 `monitor` 时必须提供 `-p` / `--port`。
5. 监视默认波特率：`115200`。退出：Ctrl+C（PowerShell 脚本会关闭串口）。

---

## 命令参考

### 通用选项

```
Usage: wsl-idf [OPTIONS] [TRAILING]...

Options:
      --about              关于
  -p, --port <PORT>        设备端口号（Windows COM，如 COM3）
  -c, --command <COMMAND>  见下表
  -h, --help
  -V, --version

Arguments:
  [TRAILING]...            monitor [波特率]
```

| `-c` 取值 | 需要 `-p` | 需要 `ESPTOOL` | 自动 `idf.py build` | 作用 |
|-----------|-----------|----------------|---------------------|------|
| `flash` | 是 | 是 | 是 | 烧录 `flash_files` 全部分区 |
| `flash-app` | 是 | 是 | 是 | 仅烧录 `app` |
| `flash-erase` | 是 | 是 | 是 | 全片擦除后完整烧录 |
| `erase` | 是 | 是 | 否 | 仅擦除 Flash |
| `merge` | 否 | 是 | 否 | 合并为单个 bin |
| `extract-bins` | 否 | 否 | 否 | 提取分区 bin 到 `flash-bins/` |

未指定 `-c`、也未指定 `monitor` 时：只打印 COM 列表。

### 烧录类内部固定参数

对 `flash` / `flash-app` / `flash-erase`，工具组装的 esptool 调用中固定：

| 参数 | 默认值 |
|------|--------|
| 波特率 `-b` | `1152000` |
| `--before` | `default-reset` |
| `--after` | `hard-reset` |
| `--flash-mode` | `dio` |
| 芯片 `-c` | 来自 `build/flasher_args.json` → `extra_esptool_args.chip` |
| 偏移与文件 | 来自同文件的 `flash_files` 或 `app` |

一般无需手动覆盖上述参数。

---

### flash

**用途**：编译后按构建产物完整烧录（bootloader、分区表、app 等）。

**入口**：`wsl-idf -c flash -p <COM>`

**前置条件**：

1. 已 `source` ESP-IDF
2. 已设置 `ESPTOOL`
3. 当前目录为项目根目录
4. 指定正确 COM 口

**操作流程**：

1. 执行 `idf.py build`
2. 读取 `build/flasher_args.json`
3. 将项目路径转为 Windows 路径，调用 esptool `write-flash`

**示例**：

```bash
wsl-idf -c flash -p COM3
wsl-idf -c flash -p COM3 monitor
```

**限制与风险**：会改写设备 Flash 中对应分区内容；烧录前请确认端口，避免写错板子。

---

### flash-app

**用途**：仅烧录应用分区，适合未改分区表 / Bootloader 时的迭代。

**入口**：`wsl-idf -c flash-app -p <COM>`

**前置条件**：同 flash；且 `flasher_args.json` 含 `app` 字段（`offset`、`file`）。

**操作流程**：`idf.py build` → 仅烧录 `app` 条目。

**示例**：

```bash
wsl-idf -c flash-app -p COM3 monitor
```

**限制**：缺少 `app` 字段时报错 `flasher_args.json 中缺少 app 字段`，可改用 `flash`。

---

### flash-erase

**用途**：全片擦除后完整烧录（换分区表、清空 NVS 等场景）。

**入口**：`wsl-idf -c flash-erase -p <COM>`

**前置条件**：同 flash。

**操作流程**：`idf.py build` → `write-flash ... --erase-all`。

**限制与风险**：**破坏性操作**。`--erase-all` 会擦除整片 Flash，设备上全部数据丢失。确认端口与目标板后再执行。

---

### erase

**用途**：仅擦除 Flash，不编译、不烧录。

**入口**：`wsl-idf -c erase -p <COM>`

**前置条件**：`ESPTOOL`、COM 口；不依赖 `flasher_args.json`。

**操作流程**：调用 esptool `erase-flash`（波特率 `1152000`）。

**限制与风险**：**破坏性操作**，整片擦除，不可恢复。

---

### merge

**用途**：将各分区 bin 合并为单个固件文件（量产或离线烧录准备）。

**入口**：`wsl-idf -c merge`

**前置条件**：

1. `ESPTOOL` 已设置
2. 已存在 `build/flasher_args.json` 及对应 bin（需事先 `idf.py build`；本命令**不会**自动 build）
3. 不需要 `-p`

**输出**：项目根目录 `full-<当前目录名>.bin`  
例如目录名为 `my-esp-project` → `full-my-esp-project.bin`。

**示例**：

```bash
idf.py build   # 若尚未构建
wsl-idf -c merge
```

**限制**：依赖当前目录名作为输出文件名的一部分；目录名异常时可能失败。

---

### extract-bins

**用途**：从 `build/` 按 `flash_files` 复制各分区 bin 到 `flash-bins/`，文件名附带烧录地址，便于按偏移单独使用。

**入口**：`wsl-idf -c extract-bins`

**前置条件**：

1. 存在 `build/flasher_args.json`，且 `flash_files` 非空
2. 各源 bin 已存在于 `build/` 下（本命令**不会**自动 `idf.py build`）
3. 不需要 `-p`、不需要 `ESPTOOL`

**操作流程**：

1. 若已存在 `flash-bins/`，**整目录删除后重建**
2. 按烧录地址排序复制文件
3. 打印 `源路径 -> flash-bins/目标名`

**输出命名**：

| 规则 | 示例 |
|------|------|
| `{原文件主名}_{偏移}.bin` | `bootloader_0x0.bin`、`partition-table_0x8000.bin` |

**示例**：

```bash
idf.py build
wsl-idf -c extract-bins
# 输出目录：./flash-bins/
```

**限制与风险**：每次执行会清空已有 `flash-bins/`，该目录内自定义文件会被删除。源文件缺失时报错并提示先 `idf.py build`。

---

### 串口监视（尾部参数）

**用途**：打开 Windows COM 口，将设备输出显示到当前终端（UTF-8）。

**入口**：

| 场景 | 命令 |
|------|------|
| 仅监视 | `wsl-idf -p COM3 monitor` |
| 指定波特率 | `wsl-idf -p COM3 monitor 9600` |
| 烧录后监视 | `wsl-idf -c flash -p COM3 monitor` |
| 仅刷 app 后监视 | `wsl-idf -c flash-app -p COM3 monitor 115200` |

**参数表**：

| 参数 | 含义 | 默认值 |
|------|------|--------|
| `-p` / `--port` | Windows COM 口 | 无（必填） |
| 波特率（`monitor` 后可选数字） | 串口波特率 | `115200` |

**主界面提示（PowerShell）**：打开成功后会打印端口名、波特率，以及「等待接收数据... (按 Ctrl+C 退出)」。

**前置条件**：COM 口未被其他程序占用；监视路径会走日志着色与崩溃解析逻辑。

**限制**：

- 尾部参数仅支持 `monitor` 或 `monitor <波特率>`，其它写法报错：`未知尾部参数 ...`
- 波特率必须为合法整数
- 本工具监视为只读查看，不提供 `idf.py monitor` 的交互式菜单/快捷键生态

---

## 串口监视：日志着色

识别 ESP-IDF 标准日志行（如 `I (637787) wifi: ...`），按等级着色（ANSI，配色接近 `idf.py monitor`）：

| 等级 | 字符 | 颜色 |
|------|------|------|
| Error | E | 红 |
| Warn | W | 黄 |
| Info | I | 绿 |
| Debug | D | 青 |
| Verbose | V | 灰 |

### 自定义高亮：`@@@`

Info 级日志正文以 `@@@` 开头时，以紫色显示，并去掉正文前的 `@@@`：

```
# 设备输出
I (1000) LOG_DEMO: @@@自定义高亮

# 终端显示（紫色，无 @@@ 前缀）
I (1000) LOG_DEMO: 自定义高亮
```

固件侧示例：

```c
ESP_LOGI(TAG, "@@@%s", "自定义高亮");
```

非 ESP-IDF 格式的行原样输出（含 PowerShell 的打开/关闭提示）。

---

## 串口监视：崩溃地址解析

监视过程中检测常见崩溃日志块，结束后收集十六进制地址，调用 WSL 中的 `addr2line`，在输出中插入：

```
--- wsl-idf addr2line ---
```

其后为函数名、文件、行号等（取决于 ELF 与调试信息）。

### 触发（进入收集）

行内包含任一：

- `Guru Meditation Error`
- `register dump`
- `Backtrace:`
- `abort() was called`
- `Stack memory:`
- `Task watchdog got triggered`
- `assert failed`
- `Panic handler`

### 结束（触发解析）

行内包含任一：

- `Please enable CONFIG_ESP_SYSTEM_USE_FRAME_POINTER`
- `ELF file SHA256`
- `Rebooting...`
- `Backtrace stopped`

### 地址规则

- 收集行内 `0x` 十六进制地址（长度足够时）
- `Backtrace:` 行中 `PC:SP` 只取 PC
- 忽略占位地址：`0x00000000`、`0xdeadc0de`、`0xdeadbeef`、`0xabababab`、`0xcdcdcdcd`
- 调用：`addr2line -pfiaC -e <elf> <addrs...>`

### ELF 与 addr2line 查找顺序

| 资源 | 顺序 |
|------|------|
| ELF | ① `build/project_description.json` 的 `app_elf`；② `build/<项目目录名>.elf`；③ `build/` 下任意 `.elf` |
| addr2line | ① `build/CMakeCache.txt` 中 `CMAKE_ADDR2LINE`；② PATH 中 `riscv32-esp-elf-addr2line`、`xtensa-esp32s3-elf-addr2line`、`xtensa-esp32s2-elf-addr2line`、`xtensa-esp32-elf-addr2line`、`xtensa-esp-elf-addr2line` |

未就绪时启动监视会提示（不影响看日志），例如：

```
wsl-idf: 崩溃地址自动解析未启用（未找到 build/*.elf）
```

**注意**：ELF 须与设备内固件同一次构建，否则行号可能不准。需要完整回溯时可在 `sdkconfig` 中启用 `CONFIG_ESP_SYSTEM_USE_FRAME_POINTER`。

---

## 数据与输出路径

| 路径 | 产生方 | 说明 |
|------|--------|------|
| `build/flasher_args.json` | ESP-IDF / `idf.py build` | 烧录偏移、文件、芯片型号 |
| `build/*.bin`、`build/*.elf` | ESP-IDF | 烧录与崩溃解析输入 |
| `full-<目录名>.bin` | `merge` | 写在**项目根目录** |
| `flash-bins/*.bin` | `extract-bins` | 每次运行会先删除整个 `flash-bins/` |

本工具不写入独立配置文件；配置依赖环境变量 `ESPTOOL` 与命令行参数。

---

## 工作原理

```
WSL2                              Windows
┌─────────────┐                   ┌──────────────┐
│  wsl-idf    │──idf.py build──▶  │              │
│             │──wslpath -w──▶    │  esptool.exe │──▶ COMx ──▶ ESP
│             │──powershell.exe─▶ │  SerialPort  │──▶ 串口数据回传
│  addr2line  │◀─崩溃时解析 ELF   └──────────────┘
└─────────────┘
```

1. 烧录列表与芯片型号来自 `build/flasher_args.json`。
2. bin 路径以 Windows 形式传给 esptool（`项目Windows路径\build\...`）。
3. 监视由 PowerShell `System.IO.Ports.SerialPort` 读 COM；stdout 经 wsl-idf 着色与崩溃解析后显示。

---

## 与 idf.py 对照

| 目的 | idf.py（原生 Linux USB） | wsl-idf（WSL + Windows COM） |
|------|--------------------------|------------------------------|
| 编译 | `idf.py build` | `flash` / `flash-app` / `flash-erase` 内自动执行；`merge` / `erase` / `extract-bins` 不会 |
| 烧录 | `idf.py flash` | `wsl-idf -c flash -p COMx` |
| 仅 app | `idf.py app-flash` | `wsl-idf -c flash-app -p COMx` |
| 擦除后烧录 | `idf.py erase-flash flash` | `wsl-idf -c flash-erase -p COMx` |
| 仅擦除 | `idf.py erase-flash` | `wsl-idf -c erase -p COMx` |
| 监视 | `idf.py monitor` | `wsl-idf -p COMx monitor [波特率]` |
| 合并 bin | 自行拼 esptool | `wsl-idf -c merge` |
| 按偏移导出 bin | — | `wsl-idf -c extract-bins` |

`wsl-idf` **不替代** ESP-IDF 项目管理（`menuconfig`、`clean`、组件依赖等），这些仍用 `idf.py`。

---

## 常见问题

### 未设置 ESPTOOL

```
未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）
```

按 [配置](#环境变量-esptool烧录--擦除--合并必需) 设置。

### 找不到 flasher_args.json

在项目根执行 `idf.py build`，或使用会自动 build 的 `flash` / `flash-app` / `flash-erase`。

### flash-app 缺少 app 字段

确认工程可正常完整构建；或改用 `-c flash`。

### 串口打开失败

- `-p` 与设备管理器 / 列表中的 COM 一致
- 关闭占用该口的其他监视器或串口工具
- 重新插拔 USB 或更换线缆

### wslpath 失败

确认在 WSL 内运行，且当前路径对 `wslpath -w` 合法。

### 崩溃解析未启用或行号不对

- 在项目根运行监视，且 `build/*.elf` 存在
- 已 `source export.sh`，addr2line 可用
- 设备固件与当前 ELF 为同一次 build

### COM 列表为空

- 检查 Windows 驱动与 USB
- 在 WSL 自测：`powershell.exe -Command "Get-WmiObject Win32_SerialPort"`

### extract-bins 清空目录

属预期行为：每次运行删除并重建 `flash-bins/`。

---

## 功能状态

| 功能 | 状态 |
|------|------|
| flash / flash-app / flash-erase / erase | 已实现 |
| merge | 已实现 |
| extract-bins | 已实现 |
| monitor（着色、`@@@` 高亮） | 已实现 |
| 崩溃 addr2line 自动解析 | 已实现（条件不足时降级提示） |
| COM 口枚举 | 已实现 |
| `--about` | 已实现 |
| 可配置烧录波特率 / flash mode 等 CLI 覆盖项 | 未实现（代码内固定） |
| 监视侧交互式发送 / idf.py monitor 快捷键 | 未实现 |

VS Code 扩展能力见 [`vsc-plugin/README.md`](./vsc-plugin/README.md)，不在上表展开。
