# wsl-idf

[中文](./README.MD) | **English**

wsl-idf helps you develop ESP-IDF projects in WSL2 while your board is connected via Windows COM ports.
Build in WSL with idf.py; flash, erase, merge firmware, and open a serial monitor through Windows esptool.exe — no manual path juggling.
Commands mirror idf.py flash / monitor, with log coloring and automatic crash addr2line decoding.

---

## Table of Contents

- [Background & Use Cases](#background--use-cases)
- [Features](#features)
- [Requirements](#requirements)
- [Installation & Build](#installation--build)
- [Configuration](#configuration)
- [Quick Start](#quick-start)
- [Command Reference](#command-reference)
- [Serial Monitor](#serial-monitor)
- [Automatic Crash Address Decoding](#automatic-crash-address-decoding)
- [How It Works](#how-it-works)
- [Typical Workflows](#typical-workflows)
- [FAQ](#faq)

---

## Background & Use Cases

The official ESP-IDF toolchain runs smoothly on Linux/WSL, but USB-to-serial adapters are usually attached to the **Windows host**, and WSL cannot access `COM` ports directly. Common workarounds:

- Install a separate ESP-IDF / esptool stack on Windows; or
- Manually switch terminals, copy paths, and stitch together esptool arguments.

**wsl-idf** lets you **build with `idf.py` in WSL** while **flashing and monitoring go through Windows `esptool.exe` and COM ports**, with a CLI experience close to `idf.py flash` / `idf.py monitor`.

Good fit when:

| Scenario | Description |
|----------|-------------|
| WSL2 + ESP-IDF development | Code and builds in WSL; USB hardware on Windows |
| Windows esptool already available | No need to set up USB serial forwarding in WSL |
| Daily flash + debug | One command for flash / app-only flash / erase / merge / monitor |

---

## Features

### Firmware Operations

| Command | Description |
|---------|-------------|
| **flash** | Runs `idf.py build`, then flashes all partitions from `build/flasher_args.json` |
| **flash-app** | Runs `idf.py build`, then flashes only the `app` partition (faster iteration when partition table/bootloader unchanged) |
| **flash-erase** | Runs `idf.py build`, then full-chip erase (`--erase-all`) followed by a complete flash |
| **erase** | Erases flash only; no build, no flash |
| **merge** | Merges partition bins into a single `full-<project-name>.bin` (no COM port required) |

### Serial Monitor

- Opens COM ports via Windows PowerShell with UTF-8 output
- **Automatic ESP-IDF log coloring** (E/W/I/D/V levels, similar palette to `idf.py monitor`)
- **`@@@` prefix custom highlight** (Info logs shown in purple with the marker stripped)
- **Crash detection + automatic addr2line decoding** (Guru Meditation, watchdog, assert, etc.)

### Other

- Lists available **COM ports** on Windows at startup to help pick `-p`
- Reads chip type, flash offsets, and file paths automatically—no manual esptool args
- Converts WSL paths with `wslpath -w` before passing them to esptool on Windows

---

## Requirements

### WSL Side

- WSL2 (Ubuntu or similar recommended)
- **ESP-IDF** installed and sourced (includes `idf.py`)
- **Rust toolchain** (to build this tool; not needed if using a prebuilt binary)
- Current working directory must be the **ESP-IDF project root** (with `CMakeLists.txt`, `sdkconfig`, etc.)

### Windows Side

- **esptool** executable (e.g. `esptool.exe` or the official `esptool-windows-amd64` package)
- **PowerShell** (invoked from WSL via `powershell.exe`)
- ESP board USB drivers installed; COM port visible in Device Manager

### Crash Decoding (Optional)

- Project has been built with `idf.py build`; `build/*.elf` exists
- Matching `*-addr2line` available in WSL (usually after `source $IDF_PATH/export.sh`)

---

## Installation & Build

```bash
git clone <repo-url> ~/rust-project/wsl-idf
cd ~/rust-project/wsl-idf
cargo build --release
```

Binary: `target/release/wsl-idf`

Recommended shell alias (`~/.bashrc` or `~/.zshrc`):

```bash
alias wsl-idf='$HOME/rust-project/wsl-idf/target/release/wsl-idf'
```

Verify:

```bash
wsl-idf --help
wsl-idf --version
```

---

## Configuration

### 1. ESPTOOL Environment Variable (Required)

Points to the **Windows** esptool executable. Use Windows-style paths from WSL:

```bash
export ESPTOOL='D:\\esptool-windows-amd64\\esptool.exe'
```

Notes:

- Path must be reachable from Windows (`C:\`, `D:\`, etc.)
- Escape backslashes as `\\` in the shell, or use appropriate quoting
- Add the `export` to `~/.bashrc` / `~/.zshrc` to persist

If unset, the tool errors with: `未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）`

### 2. ESP-IDF Environment (WSL)

Before working on an ESP project in a new terminal:

```bash
source ~/esp/esp-idf/export.sh   # adjust to your install path
```

Flashing requires a working `idf.py build`; crash decoding needs `riscv32-esp-elf-addr2line` or `xtensa-*-addr2line` on PATH.

### 3. Confirm COM Port

After connecting the board, check **Device Manager → Ports (COM & LPT)** on Windows for the port number, e.g. `COM3`, `COM20`.

Every `wsl-idf` run prints currently visible serial ports (from `Win32_SerialPort`), for example:

```
COM1
COM3
COM20
```

---

## Quick Start

```bash
# 1. Enter ESP-IDF project root
cd ~/projects/my-esp-project

# 2. Load IDF environment
source ~/esp/esp-idf/export.sh

# 3. Confirm ESPTOOL is set
echo $ESPTOOL

# 4. Build and flash (runs idf.py build internally)
wsl-idf -c flash -p COM3

# 5. Flash then open serial monitor (default 115200)
wsl-idf -c flash -p COM3 monitor

# 6. Monitor only (no flash)
wsl-idf -p COM3 monitor
```

---

## Command Reference

### Global Options

```
Usage: wsl-idf [OPTIONS] [TRAILING]...

Options:
  -p, --port <PORT>        Device port (Windows COM port, e.g. COM3)
  -c, --command <COMMAND>  Flash subcommand (see table below)
  -h, --help               Help
  -V, --version            Version

Arguments:
  [TRAILING]...            Serial monitor after command: monitor [baud]
```

### Subcommands `-c / --command`

| Subcommand | Requires `-p` | Auto `idf.py build` | Action |
|------------|---------------|---------------------|--------|
| `flash` | Yes | Yes | Flash bootloader, partition table, app, and all partitions from `flasher_args.json` |
| `flash-app` | Yes | Yes | Flash app partition only |
| `flash-erase` | Yes | Yes | Full-chip erase then complete flash |
| `erase` | Yes | No | Run `erase-flash` only |
| `merge` | No | No | Merge bins to `full-<project-name>.bin` in project directory |

### Flash Command Examples

```bash
# Full flash
wsl-idf -c flash -p COM3

# App-only flash (faster dev iteration)
wsl-idf -c flash-app -p COM3

# Erase entire flash then re-flash (partition table change, clear NVS, etc.)
wsl-idf -c flash-erase -p COM3

# Erase only, no flash
wsl-idf -c erase -p COM3

# Merge firmware (single file for production or OTA prep)
wsl-idf -c merge
# Output example: ~/projects/my-esp-project/full-my-esp-project.bin
```

### Fixed Flash Parameters (Internal)

For `flash` / `flash-app` / `flash-erase`, wsl-idf assembles esptool calls like (offsets/partitions from `build/flasher_args.json`):

- Baud rate: `-b 1152000`
- Reset: `--before default-reset --after hard-reset`
- Flash mode: `--flash-mode dio`
- Chip: from `extra_esptool_args.chip` in `flasher_args.json`

You generally **do not** need to specify these manually.

### List Ports / Monitor Only

```bash
# Print COM port list only (no -c → no other action)
wsl-idf

# Open serial monitor, default 115200
wsl-idf -p COM3 monitor

# Custom baud rate
wsl-idf -p COM3 monitor 9600
```

### Flash + Monitor Combinations

Append `monitor` or `monitor <baud>` at the end. The `-c` action runs **first**, then monitoring starts. `-p` is required when using `monitor`.

| Scenario | Command |
|----------|---------|
| Flash then monitor | `wsl-idf -c flash -p COM3 monitor` |
| Flash then monitor (9600) | `wsl-idf -c flash -p COM3 monitor 9600` |
| App flash then monitor | `wsl-idf -c flash-app -p COM3 monitor 115200` |
| Erase, flash, then monitor | `wsl-idf -c flash-erase -p COM3 monitor` |
| Erase then monitor | `wsl-idf -c erase -p COM3 monitor 115200` |
| Monitor only | `wsl-idf -p COM3 monitor` |

Press **Ctrl+C** to exit monitoring; the PowerShell script closes the serial port.

---

## Serial Monitor

### Log Coloring

Monitor output recognizes standard ESP-IDF log format, for example:

```
I (637787) wifi: station connected
E (640797) main: [ERROR] connection failed
W (643817) heap: memory usage high
```

Levels and ANSI colors:

| Level | Char | Color |
|-------|------|-------|
| Error | E | Red |
| Warn | W | Yellow |
| Info | I | Green |
| Debug | D | Cyan |
| Verbose | V | Gray |

### Custom Highlight: `@@@` Marker

If an Info-level log body starts with `@@@`, it is shown in **purple** and the prefix is stripped—useful for highlighting key lines in noisy output:

```
# Device prints
I (1000) LOG_DEMO: @@@custom highlight

# Terminal shows (purple, no @@@)
I (1000) LOG_DEMO: custom highlight
```

In firmware:

```c
ESP_LOGI(TAG, "@@@%s", "custom highlight");
```

---

## Automatic Crash Address Decoding

While monitoring, wsl-idf watches for ESP crash-related log lines. After a crash block ends, it collects **hex addresses**, runs **addr2line** in WSL, and inserts output like:

```
--- wsl-idf addr2line ---
<function names, file paths, line numbers, etc.>
```

### Start Triggers

Enters “crash collection” when a line contains any of:

- `Guru Meditation Error`
- `register dump`
- `Backtrace:`
- `abort() was called`
- `Stack memory:`
- `Task watchdog got triggered`
- `assert failed`
- `Panic handler`

### End Triggers

- `Please enable CONFIG_ESP_SYSTEM_USE_FRAME_POINTER`
- `ELF file SHA256`
- `Rebooting...`
- `Backtrace stopped`

### Address Sources

- `0x........` addresses in register dumps and exception info
- **PC** part of `PC:SP` pairs on `Backtrace:` lines
- Placeholder addresses ignored: `0x00000000`, `0xdeadc0de`, `0xdeadbeef`, etc.

### Prerequisites

1. Run monitor from the **ESP-IDF project root** (needs `build/`)
2. ELF present: prefer `app_elf` in `build/project_description.json`, else `build/<project-name>.elf`
3. addr2line found: prefer `CMAKE_ADDR2LINE` in `build/CMakeCache.txt`, else search PATH for `riscv32-esp-elf-addr2line`, `xtensa-esp32-elf-addr2line`, etc.

If prerequisites are missing, monitor still works but prints a warning at startup:

```
wsl-idf: 崩溃地址自动解析未启用（未找到 build/*.elf）
```

### Tips

- Ensure **`idf.py build` matches the firmware on the device** before reproducing a crash, or line numbers may be wrong
- Enable `CONFIG_ESP_SYSTEM_USE_FRAME_POINTER` in `sdkconfig` for fuller backtraces

---

## How It Works

```
┌─────────────────────────────────────────────────────────────┐
│                        WSL2                                 │
│  ┌──────────────┐    ┌─────────────────┐    ┌──────────────┐  │
│  │   wsl-idf    │───▶│  idf.py build   │    │  addr2line   │  │
│  │   (Rust)     │    │  (before flash) │    │  (crashes)   │  │
│  └──────┬───────┘    └─────────────────┘    └──────────────┘  │
│         │ reads build/flasher_args.json                       │
│         │ wslpath -w for project path                         │
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
│  │  esptool.exe │────────▶│  COMx USB    │──▶ ESP chip      │
│  └──────────────┘         └──────────────┘                  │
│  ┌──────────────┐                                            │
│  │ SerialPort   │◀── monitor reads COM ──▶ colored WSL output  │
│  └──────────────┘                                            │
└─────────────────────────────────────────────────────────────┘
```

Key points:

1. **Build info**: Flash layout comes from ESP-IDF’s `build/flasher_args.json` (`flash_files`, `app`, `extra_esptool_args.chip`).
2. **Paths**: WSL project path is converted via `wslpath -w` to `D:\...` so Windows esptool can read bins under `build\`.
3. **Monitor**: PowerShell `System.IO.Ports.SerialPort` reads the COM port; wsl-idf colors stdout and runs crash decoding.

---

## Typical Workflows

### Daily Development (App Code Changes)

```bash
cd ~/projects/my-esp-project
source ~/esp/esp-idf/export.sh

# Edit code → app-only flash + monitor
wsl-idf -c flash-app -p COM3 monitor
```

### Partition Table / Bootloader / First Flash

```bash
wsl-idf -c flash -p COM3 monitor
# Or wipe entire flash first:
wsl-idf -c flash-erase -p COM3 monitor
```

### Export Merged Firmware

```bash
# Requires build/flasher_args.json (at least one build)
wsl-idf -c merge
ls -la full-*.bin
```

### Debug a Crash

```bash
# Build first; ELF must match firmware on device
wsl-idf -p COM3 monitor
# Reproduce crash — addr2line output is appended automatically
```

---

## FAQ

### ESPTOOL Not Set

```
未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）
```

Fix: Set `ESPTOOL` to the full Windows path to esptool as described in [Configuration](#1-esptool-environment-variable-required).

### flasher_args.json Not Found

```
No such file or directory ... build/flasher_args.json
```

Fix: Run `idf.py build` from the project root, or use `wsl-idf -c flash` (builds automatically).

### flash-app Missing app Field

```
flasher_args.json 中缺少 app 字段
```

Fix: Confirm ESP-IDF version and project config; run a full `idf.py build`. Some special builds may omit the app entry—use `flash` instead.

### Serial Port Open Failed

PowerShell shows `串口打开失败`:

- Confirm `-p` matches Device Manager (case usually irrelevant; match the listed output)
- Close other apps using the COM port (Arduino IDE, another monitor, serial tools)
- Replug USB or try a different cable

### wslpath Failed

```
wslpath 失败（退出码 ...）
```

Fix: Run inside WSL with a valid current directory; `cd` to the project root if needed.

### Crash Decoding Disabled or Wrong Line Numbers

- Run from project root with `build/*.elf` present
- `source export.sh` so addr2line is on PATH
- Re-flash firmware from the same build as the ELF before reproducing the crash

### Empty or Missing COM List

- Check Windows drivers and USB connection
- WSL must invoke `powershell.exe`; test with `powershell.exe -Command "Get-WmiObject Win32_SerialPort"`

### merge Output Location

Merged file is written to the **current project root**: `full-<directory-name>.bin`. For a folder named `my-esp-project`, output is `full-my-esp-project.bin`.

---

## Comparison with idf.py

| Goal | idf.py (native Linux USB) | wsl-idf (WSL + Windows COM) |
|------|---------------------------|-----------------------------|
| Build | `idf.py build` | Auto-build inside `flash*` subcommands; not for `merge`/`erase` |
| Flash | `idf.py flash` | `wsl-idf -c flash -p COMx` |
| App only | `idf.py app-flash` | `wsl-idf -c flash-app -p COMx` |
| Erase + flash | `idf.py erase-flash flash` | `wsl-idf -c flash-erase -p COMx` |
| Monitor | `idf.py monitor` | `wsl-idf -p COMx monitor [baud]` |
| Merge bin | Manual esptool args | `wsl-idf -c merge` |

wsl-idf **does not replace** full ESP-IDF project management (menuconfig, clean, component deps, etc.)—use `idf.py` in WSL for those.

---

## License

See the LICENSE file in the repository if present. This is an ESP-IDF development helper; also follow Espressif and esptool licenses and documentation.

---

## Links

- [ESP-IDF Programming Guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32/)
- [esptool Documentation](https://docs.espressif.com/projects/esptool/en/latest/)
- [WSL Documentation](https://learn.microsoft.com/en-us/windows/wsl/)
