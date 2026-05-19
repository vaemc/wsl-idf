# wsl-idf

在 WSL 中直接烧录 ESP-IDF 项目固件。

## 配置 esptool

```bash
export ESPTOOL='D:\\esptool-windows-amd64\\esptool.exe'
```

`esptool.exe` 必须放在 Windows 系统上。

## 配置 wsl-idf

```bash
alias wsl-idf='$HOME/rust-project/wsl-idf/target/release/wsl-idf'
```

## 用法

在 ESP-IDF 项目中先运行 `idf.py build`，编译成功后再执行：

| 操作 | 命令 |
|------|------|
| 烧录固件 | `wsl-idf -c flash -p COM1` |
| 合并固件 | `wsl-idf -c merge -p COM1` |
| 仅烧录 app | `wsl-idf -c flash-app -p COM1` |
| 擦除后再烧录 | `wsl-idf -c flash-erase -p COM1` |
| 擦除 | `wsl-idf -c erase -p COM1` |

### 串口监视（monitor）

在任意命令末尾追加 `monitor` 可在操作完成后打开串口监视；波特率可选，默认 `115200`。

| 操作 | 命令 |
|------|------|
| 烧录后监视 | `wsl-idf -c flash -p COM1 monitor 115200` |
| 合并后监视 | `wsl-idf -c merge -p COM1 monitor 9600` |
| 仅烧录 app 后监视 | `wsl-idf -c flash-app -p COM1 monitor 115200` |
| 擦除后烧录并监视 | `wsl-idf -c flash-erase -p COM1 monitor` |
| 擦除后监视 | `wsl-idf -c erase -p COM1 monitor 115200` |
| 仅监视 | `wsl-idf -p COM1 monitor` |

- `monitor`：执行串口监视（按 Ctrl+C 退出）
- `monitor <波特率>`：指定波特率，例如 `monitor 9600`
- 使用 `monitor` 时必须通过 `-p` 指定端口
