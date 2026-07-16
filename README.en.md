# wsl-idf

[中文](./README.MD) | **English**

A command-line tool (Rust / clap) for flashing, erasing, merging firmware, and serial monitoring of ESP-IDF projects developed in WSL2, using Windows-side `esptool` and COM ports.

This repository also includes a VS Code status-bar extension; see [`vsc-plugin/`](./vsc-plugin/README.md). This document covers the CLI only.

---

## Table of Contents

- [Overview](#overview)
- [Feature Summary](#feature-summary)
- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Global Behavior](#global-behavior)
- [Command Reference](#command-reference)
  - [Common Options](#common-options)
  - [flash](#flash)
  - [flash-app](#flash-app)
  - [flash-erase](#flash-erase)
  - [erase](#erase)
  - [merge](#merge)
  - [extract-bins](#extract-bins)
  - [Serial Monitor (Trailing Args)](#serial-monitor-trailing-args)
- [Monitor: Log Coloring](#monitor-log-coloring)
- [Monitor: Crash Address Decoding](#monitor-crash-address-decoding)
- [Output Paths](#output-paths)
- [How It Works](#how-it-works)
- [Comparison with idf.py](#comparison-with-idfpy)
- [FAQ](#faq)
- [Feature Status](#feature-status)

---

## Overview

ESP-IDF builds fine in WSL, but USB serial adapters usually appear as Windows `COMx` ports that WSL cannot open directly. `wsl-idf` runs `idf.py build` when needed, reads `build/flasher_args.json`, converts paths with `wslpath -w`, and invokes Windows `esptool` / serial I/O via `powershell.exe`.

| Area | Contents |
|------|----------|
| Firmware ops | `flash` / `flash-app` / `flash-erase` / `erase` / `merge` / `extract-bins` |
| Serial monitor | Trailing `monitor [baud]`; PowerShell reads COM; local coloring and crash decode |
| Helpers | Lists COM ports on startup; `--about` prints author and repo |

Working directory must be the **ESP-IDF project root**.

---

## Feature Summary

| Capability | Entry | Notes |
|------------|-------|-------|
| Full flash | `-c flash -p COMx` | Runs `idf.py build`, then flashes all partitions from `flasher_args.json` |
| App-only flash | `-c flash-app -p COMx` | Build, then flash `app` only |
| Erase then flash | `-c flash-erase -p COMx` | Build, then `write-flash ... --erase-all` |
| Erase only | `-c erase -p COMx` | `erase-flash`; no build, no flash |
| Merge bins | `-c merge` | Writes `full-<dirname>.bin` at project root; no port |
| Extract partition bins | `-c extract-bins` | Copies into `flash-bins/` with offset suffix; no port or `ESPTOOL` |
| Serial monitor | `-p COMx monitor [baud]` | Alone or after a `-c` command |
| List COM ports | (no `-c`, no monitor) | Prints Windows serial ports only |
| About | `--about` | Author, copyright, repo; then exit |

---

## Requirements

### WSL side

| Item | Requirement |
|------|-------------|
| Environment | WSL2 (Ubuntu or similar recommended) |
| ESP-IDF | Installed and `source`d in the current shell (`idf.py` available) |
| Working directory | ESP-IDF project root |
| Building this tool | Rust toolchain (not needed for a prebuilt binary) |
| Path conversion | `wslpath` available |
| Crash decode (optional) | `build/*.elf` present; matching `*-addr2line` findable |

### Windows side

| Item | Requirement |
|------|-------------|
| esptool | Executable (e.g. `esptool.exe`); path in `ESPTOOL` |
| PowerShell | Invokable as `powershell.exe` from WSL |
| Drivers | Board USB drivers installed; COM visible in Device Manager |

Commands `flash` / `flash-app` / `flash-erase` / `erase` / `merge` require `ESPTOOL`. `extract-bins` and monitor-only do not.

---

## Installation

Build from source:

```bash
git clone https://github.com/vaemc/wsl-idf.git
cd wsl-idf
cargo build --release
```

Binary: `target/release/wsl-idf`.

Optional alias in `~/.bashrc` / `~/.zshrc`:

```bash
alias wsl-idf='$HOME/path/to/wsl-idf/target/release/wsl-idf'
```

Verify:

```bash
wsl-idf --help
wsl-idf --version
```

Current version (from the binary): `0.1.0`.

---

## Configuration

### Environment variable `ESPTOOL` (required for flash / erase / merge)

Windows path to esptool, written in the WSL shell:

```bash
export ESPTOOL='D:\\esptool-windows-amd64\\esptool.exe'
```

| Item | Notes |
|------|-------|
| Location | Must be reachable from Windows (`C:\`, `D:\`, …) |
| Backslashes | Use `\\` in the shell, or suitable quoting |
| Persist | Add to `~/.bashrc` / `~/.zshrc` |
| If unset | `未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）` |

### ESP-IDF environment

```bash
source ~/esp/esp-idf/export.sh   # adjust to your install path
```

### Confirm COM port

1. Windows Device Manager → Ports (COM & LPT)
2. Or run `wsl-idf` with no subcommand to list `Win32_SerialPort` DeviceIDs

---

## Global Behavior

1. Except for `--about`, every run lists Windows COM ports first.
2. `--about` prints branding and exits; no port list, no other actions.
3. Trailing `monitor` or `monitor <baud>`: run the `-c` action (if any), then open the monitor.
4. `monitor` requires `-p` / `--port`.
5. Default monitor baud: `115200`. Exit with Ctrl+C (PowerShell closes the port).

---

## Command Reference

### Common Options

```
Usage: wsl-idf [OPTIONS] [TRAILING]...

Options:
      --about              About
  -p, --port <PORT>        Device port (Windows COM, e.g. COM3)
  -c, --command <COMMAND>  See table below
  -h, --help
  -V, --version

Arguments:
  [TRAILING]...            monitor [baud]
```

| `-c` value | Needs `-p` | Needs `ESPTOOL` | Auto `idf.py build` | Role |
|------------|------------|-----------------|---------------------|------|
| `flash` | yes | yes | yes | Flash all `flash_files` partitions |
| `flash-app` | yes | yes | yes | Flash `app` only |
| `flash-erase` | yes | yes | yes | Full erase then complete flash |
| `erase` | yes | yes | no | Erase flash only |
| `merge` | no | yes | no | Merge into one bin |
| `extract-bins` | no | no | no | Extract bins into `flash-bins/` |

With neither `-c` nor `monitor`: only print the COM list.

### Fixed flash parameters

For `flash` / `flash-app` / `flash-erase`, the assembled esptool invocation uses:

| Parameter | Default |
|-----------|---------|
| Baud `-b` | `1152000` |
| `--before` | `default-reset` |
| `--after` | `hard-reset` |
| `--flash-mode` | `dio` |
| Chip `-c` | From `build/flasher_args.json` → `extra_esptool_args.chip` |
| Offsets / files | From `flash_files` or `app` in the same file |

These are not exposed as CLI overrides.

---

### flash

**Purpose**: Build, then flash all partitions from the build artifacts.

**Entry**: `wsl-idf -c flash -p <COM>`

**Prerequisites**: ESP-IDF sourced; `ESPTOOL` set; project root; correct COM port.

**Flow**: `idf.py build` → read `flasher_args.json` → convert paths → `write-flash`.

**Examples**:

```bash
wsl-idf -c flash -p COM3
wsl-idf -c flash -p COM3 monitor
```

**Risk**: Overwrites Flash partitions on the connected device. Confirm the port first.

---

### flash-app

**Purpose**: Flash the application partition only (faster when partition table / bootloader unchanged).

**Entry**: `wsl-idf -c flash-app -p <COM>`

**Prerequisites**: Same as flash; `flasher_args.json` must contain `app` (`offset`, `file`).

**Flow**: `idf.py build` → flash `app` entry only.

**Example**:

```bash
wsl-idf -c flash-app -p COM3 monitor
```

**Limit**: Missing `app` → `flasher_args.json 中缺少 app 字段`; use `flash` instead.

---

### flash-erase

**Purpose**: Full-chip erase followed by a complete flash (partition table changes, clear NVS, etc.).

**Entry**: `wsl-idf -c flash-erase -p <COM>`

**Flow**: `idf.py build` → `write-flash ... --erase-all`.

**Risk**: **Destructive.** `--erase-all` wipes the entire Flash. Confirm the target board.

---

### erase

**Purpose**: Erase Flash only; no build, no program.

**Entry**: `wsl-idf -c erase -p <COM>`

**Prerequisites**: `ESPTOOL` and COM; does not need `flasher_args.json`.

**Flow**: esptool `erase-flash` at baud `1152000`.

**Risk**: **Destructive.** Full-chip erase; data is not recoverable.

---

### merge

**Purpose**: Merge partition bins into one firmware file.

**Entry**: `wsl-idf -c merge`

**Prerequisites**:

1. `ESPTOOL` set
2. `build/flasher_args.json` and bins already present (**does not** auto-build)
3. No `-p` required

**Output**: `full-<current-directory-name>.bin` at the project root  
e.g. folder `my-esp-project` → `full-my-esp-project.bin`.

**Example**:

```bash
idf.py build   # if not already built
wsl-idf -c merge
```

---

### extract-bins

**Purpose**: Copy each partition bin from `build/` into `flash-bins/`, with the flash offset in the filename.

**Entry**: `wsl-idf -c extract-bins`

**Prerequisites**:

1. `build/flasher_args.json` with non-empty `flash_files`
2. Source bins under `build/` (**does not** auto-build)
3. No `-p`, no `ESPTOOL`

**Flow**:

1. If `flash-bins/` exists, **delete the whole directory**, then recreate it
2. Copy files sorted by flash offset
3. Print `src -> flash-bins/dest`

**Naming**: `{original-stem}_{offset}.bin`  
e.g. `bootloader_0x0.bin`, `partition-table_0x8000.bin`

**Example**:

```bash
idf.py build
wsl-idf -c extract-bins
```

**Risk**: Each run wipes `flash-bins/`. Custom files in that directory are removed.

---

### Serial Monitor (Trailing Args)

**Purpose**: Open a Windows COM port and show device output (UTF-8) in the terminal.

| Scenario | Command |
|----------|---------|
| Monitor only | `wsl-idf -p COM3 monitor` |
| Custom baud | `wsl-idf -p COM3 monitor 9600` |
| Flash then monitor | `wsl-idf -c flash -p COM3 monitor` |
| App flash then monitor | `wsl-idf -c flash-app -p COM3 monitor 115200` |

| Parameter | Meaning | Default |
|-----------|---------|---------|
| `-p` / `--port` | Windows COM | required |
| baud after `monitor` | Serial baud rate | `115200` |

PowerShell prints port name, baud, and a Ctrl+C exit hint when the port opens.

**Limits**: Trailing args only accept `monitor` or `monitor <baud>`; invalid baud fails; no interactive send / idf.py-monitor keybindings.

---

## Monitor: Log Coloring

ESP-IDF log lines (`I (637787) wifi: ...`) are colored (ANSI, similar to `idf.py monitor`):

| Level | Char | Color |
|-------|------|-------|
| Error | E | red |
| Warn | W | yellow |
| Info | I | green |
| Debug | D | cyan |
| Verbose | V | gray |

### Custom highlight: `@@@`

Info-level message bodies starting with `@@@` are shown in purple with the prefix stripped:

```
# Device
I (1000) LOG_DEMO: @@@custom highlight

# Terminal (purple, no @@@)
I (1000) LOG_DEMO: custom highlight
```

Firmware:

```c
ESP_LOGI(TAG, "@@@%s", "custom highlight");
```

Non-ESP-IDF lines pass through unchanged.

---

## Monitor: Crash Address Decoding

On crash log blocks, addresses are collected and decoded with WSL `addr2line`. Output includes:

```
--- wsl-idf addr2line ---
```

### Start markers (enter collection)

Any of: `Guru Meditation Error`, `register dump`, `Backtrace:`, `abort() was called`, `Stack memory:`, `Task watchdog got triggered`, `assert failed`, `Panic handler`.

### End markers (run decode)

Any of: `Please enable CONFIG_ESP_SYSTEM_USE_FRAME_POINTER`, `ELF file SHA256`, `Rebooting...`, `Backtrace stopped`.

### Address rules

- Collect `0x…` hex addresses of sufficient length
- On `Backtrace:` lines, take PC from each `PC:SP` pair
- Skip: `0x00000000`, `0xdeadc0de`, `0xdeadbeef`, `0xabababab`, `0xcdcdcdcd`
- Invoke: `addr2line -pfiaC -e <elf> <addrs...>`

### Lookup order

| Resource | Order |
|----------|-------|
| ELF | ① `app_elf` in `build/project_description.json`; ② `build/<dirname>.elf`; ③ any `.elf` under `build/` |
| addr2line | ① `CMAKE_ADDR2LINE` in `build/CMakeCache.txt`; ② PATH candidates: `riscv32-esp-elf-addr2line`, `xtensa-esp32s3-elf-addr2line`, `xtensa-esp32s2-elf-addr2line`, `xtensa-esp32-elf-addr2line`, `xtensa-esp-elf-addr2line` |

If unavailable, a yellow stderr notice is printed; monitoring still works.

**Note**: ELF must match the firmware on the device. For fuller backtraces, enable `CONFIG_ESP_SYSTEM_USE_FRAME_POINTER` in `sdkconfig`.

---

## Output Paths

| Path | Produced by | Notes |
|------|-------------|-------|
| `build/flasher_args.json` | ESP-IDF / `idf.py build` | Offsets, files, chip |
| `build/*.bin`, `build/*.elf` | ESP-IDF | Flash / decode inputs |
| `full-<dirname>.bin` | `merge` | Project root |
| `flash-bins/*.bin` | `extract-bins` | Directory wiped each run |

No separate config file; uses `ESPTOOL` and CLI args.

---

## How It Works

```
WSL2                              Windows
┌─────────────┐                   ┌──────────────┐
│  wsl-idf    │──idf.py build──▶  │              │
│             │──wslpath -w──▶    │  esptool.exe │──▶ COMx ──▶ ESP
│             │──powershell.exe─▶ │  SerialPort  │──▶ serial data back
│  addr2line  │◀─crash decode     └──────────────┘
└─────────────┘
```

1. Flash list and chip come from `build/flasher_args.json`.
2. Bin paths are passed to esptool in Windows form.
3. Monitor uses PowerShell `SerialPort`; stdout is colored and crash-decoded by wsl-idf.

---

## Comparison with idf.py

| Goal | idf.py (native Linux USB) | wsl-idf (WSL + Windows COM) |
|------|---------------------------|-----------------------------|
| Build | `idf.py build` | Auto in `flash` / `flash-app` / `flash-erase`; not in `merge` / `erase` / `extract-bins` |
| Flash | `idf.py flash` | `wsl-idf -c flash -p COMx` |
| App only | `idf.py app-flash` | `wsl-idf -c flash-app -p COMx` |
| Erase + flash | `idf.py erase-flash flash` | `wsl-idf -c flash-erase -p COMx` |
| Erase only | `idf.py erase-flash` | `wsl-idf -c erase -p COMx` |
| Monitor | `idf.py monitor` | `wsl-idf -p COMx monitor [baud]` |
| Merge bin | Manual esptool | `wsl-idf -c merge` |
| Export bins by offset | — | `wsl-idf -c extract-bins` |

`wsl-idf` does **not** replace ESP-IDF project management (`menuconfig`, `clean`, components, etc.).

---

## FAQ

### ESPTOOL unset

Set `ESPTOOL` as in [Configuration](#environment-variable-esptool-required-for-flash--erase--merge).

### Missing flasher_args.json

Run `idf.py build`, or use `flash` / `flash-app` / `flash-erase` (auto-build).

### flash-app missing `app`

Complete a normal project build, or use `-c flash`.

### Serial open failed

Match `-p` to Device Manager / the listed COM; close other monitors; reseat USB.

### wslpath failed

Run inside WSL with a path `wslpath -w` accepts.

### Crash decode off / wrong lines

Project root with matching `build/*.elf`; IDF export sourced; firmware and ELF from the same build.

### Empty COM list

Check Windows drivers/USB; test: `powershell.exe -Command "Get-WmiObject Win32_SerialPort"`.

### extract-bins clears the directory

Expected: each run deletes and recreates `flash-bins/`.

---

## Feature Status

| Feature | Status |
|---------|--------|
| flash / flash-app / flash-erase / erase | Implemented |
| merge | Implemented |
| extract-bins | Implemented |
| monitor (coloring, `@@@` highlight) | Implemented |
| Crash addr2line decode | Implemented (degrades with a notice if unavailable) |
| COM enumeration | Implemented |
| `--about` | Implemented |
| CLI overrides for flash baud / flash mode | Not implemented (hardcoded) |
| Interactive TX / idf.py-monitor keybindings | Not implemented |

VS Code extension features: see [`vsc-plugin/README.md`](./vsc-plugin/README.md).
