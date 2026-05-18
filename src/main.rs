mod serial_output;

use clap::{Parser, ValueEnum};
use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

#[derive(Debug, Deserialize)]
struct FlasherArgsJson {
    flash_files: HashMap<String, String>,
    extra_esptool_args: ExtraEsptoolArgs,
}

#[derive(Debug, Deserialize)]
struct ExtraEsptoolArgs {
    chip: String,
}

#[derive(Debug, Clone)]
struct FlasherBuildInfo {
    chip: String,
    flash_args: String,
}

fn wsl_to_windows_path(wsl_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("wslpath")
        .arg("-w")
        .arg(wsl_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wslpath 失败（退出码 {}）：{}", output.status, stderr).into());
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn project_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    Ok(env::current_dir()?)
}

fn load_flasher_build_info() -> Result<FlasherBuildInfo, Box<dyn std::error::Error>> {
    let project_dir = project_dir()?;
    let windows_dir = wsl_to_windows_path(&project_dir)?;

    let json_path = project_dir.join("build/flasher_args.json");
    let root: FlasherArgsJson = serde_json::from_str(&fs::read_to_string(&json_path)?)?;

    let flash_args = root
        .flash_files
        .iter()
        .map(|(offset, file_path)| {
            let normalized = format!("build\\{}", file_path).replace('/', "\\");
            format!("{} {}\\{}", offset, windows_dir, normalized)
        })
        .collect::<Vec<_>>()
        .join(" ");

    Ok(FlasherBuildInfo {
        chip: root.extra_esptool_args.chip,
        flash_args,
    })
}

fn stream_child_stdout(child: &mut std::process::Child) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = child
        .stdout
        .take()
        .ok_or("无法获取子进程标准输出")?;

    let mut reader = io::BufReader::new(stdout);
    let mut buffer = [0u8; 1024];

    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        io::stdout().write_all(&buffer[..n])?;
    }

    Ok(())
}

fn run_powershell(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("powershell.exe")
        .args(["-Command", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    stream_child_stdout(&mut child)?;
    check_exit(child.wait()?, "powershell")
}

const MONITOR_PS_SCRIPT: &str = r#"& { chcp 65001 | Out-Null; [Console]::OutputEncoding = [Text.Encoding]::UTF8; $portName = $args[0]; $baud = [int]$args[1]; if (-not $portName) { throw "用法: 端口 波特率  例如: COM20 115200" }; $port = [System.IO.Ports.SerialPort]::new($portName, $baud, [System.IO.Ports.Parity]::None, 8, [System.IO.Ports.StopBits]::One); $port.Encoding = [Text.Encoding]::UTF8; $port.ReadTimeout = 1000; $port.WriteTimeout = 1000; try { $port.Open(); Write-Host ("{0} 已打开，波特率：{1}" -f $portName, $baud) -ForegroundColor Green; Write-Host "等待接收数据... (按 Ctrl+C 退出)" -ForegroundColor Yellow; Write-Host ("=" * 50); while ($port.IsOpen) { try { if ($port.BytesToRead -gt 0) { [Console]::Write($port.ReadExisting()) }; Start-Sleep -Milliseconds 50 } catch { Write-Host ("读取错误: " + $_.Exception.Message) -ForegroundColor Red } } } catch { Write-Host ("串口打开失败: " + $_.Exception.Message) -ForegroundColor Red } finally { if ($port -and $port.IsOpen) { $port.Close(); Write-Host ("`n串口已关闭") -ForegroundColor Yellow } } }"#;

fn monitor(port: &str, baud: u32) -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            MONITOR_PS_SCRIPT,
        ])
        .arg(port)
        .arg(baud.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    serial_output::stream_child_stdout(&mut child)?;
    check_exit(child.wait()?, "monitor")
}

fn check_exit(status: ExitStatus, label: &str) -> Result<(), Box<dyn std::error::Error>> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{label} 退出码：{}", status).into())
    }
}

fn require_port(port: &Option<String>) -> Result<&str, Box<dyn std::error::Error>> {
    port.as_deref()
        .ok_or("该命令需要指定端口，请使用 -p/--port".into())
}

const DEFAULT_MONITOR_BAUD: u32 = 115200;

fn parse_monitor_trailing(trailing: &[String]) -> Result<Option<u32>, Box<dyn std::error::Error>> {
    match trailing {
        [] => Ok(None),
        [word] if word == "monitor" => Ok(Some(DEFAULT_MONITOR_BAUD)),
        [word, baud] if word == "monitor" => baud
            .parse::<u32>()
            .map(Some)
            .map_err(|_| format!("无效的波特率: {baud}").into()),
        _ => Err(format!(
            "未知尾部参数: {:?}，用法: monitor [波特率]",
            trailing
        )
        .into()),
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "设备端口号")]
    port: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        help = "flash(烧录) erase(擦除) merge(合并) flashx(擦除后烧录)"
    )]
    command: Option<EsptoolCommand>,

    /// 命令完成后串口监视，用法: monitor [波特率]
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 0..=2)]
    trailing: Vec<String>,
}

#[derive(Clone, Debug, ValueEnum)]
enum EsptoolCommand {
    /// 烧录固件
    Flash,
    /// 擦除 Flash
    Erase,
    /// 合并 bin
    Merge,
    /// 擦除后烧录
    Flashx,
}

fn flash(esptool_path: &str, port: &str, erase: bool) -> Result<(), Box<dyn std::error::Error>> {
    let info = load_flasher_build_info()?;
    let mut cmd = format!(
        "{esptool_path} -p {port} -c {} -b 1152000 --before default-reset --after hard-reset write-flash --flash-mode dio {}",
        info.chip, info.flash_args
    );
    if erase {
        cmd.push_str(" --erase-all");
    }
    run_powershell(&cmd)
}

fn erase(esptool_path: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    run_powershell(&format!(
        "{esptool_path} -p {port} -b 1152000 erase-flash"
    ))
}

fn merge(esptool_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = project_dir()?;
    let info = load_flasher_build_info()?;
    let project_name = project_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("无法解析当前目录名")?;

    run_powershell(&format!(
        "{esptool_path} -c {} merge-bin -o {}/full-{project_name}.bin {}",
        info.chip,
        project_dir.display(),
        info.flash_args
    ))
}

fn port_list() -> Result<(), Box<dyn std::error::Error>> {
    run_powershell(
        "Get-WmiObject -Class Win32_SerialPort | Select-Object -ExpandProperty DeviceID",
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    port_list()?;

    let esptool_path = env::var("ESPTOOL").map_err(|_| "未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）")?;
    let args = Args::parse();
    let monitor_baud = parse_monitor_trailing(&args.trailing)?;

    match args.command {
        Some(EsptoolCommand::Flash) => {
            flash(&esptool_path, require_port(&args.port)?, false)?;
        }
        Some(EsptoolCommand::Erase) => {
            erase(&esptool_path, require_port(&args.port)?)?;
        }
        Some(EsptoolCommand::Merge) => merge(&esptool_path)?,
        Some(EsptoolCommand::Flashx) => {
            flash(&esptool_path, require_port(&args.port)?, true)?;
        }
        None => {}
    }

    if let Some(baud) = monitor_baud {
        monitor(require_port(&args.port)?, baud)?;
    }

    Ok(())
}
