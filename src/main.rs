// Copyright (c) 2025-2026 wheat. All rights reserved.
// https://github.com/vaemc/wsl-idf

mod branding;
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
    app: Option<FlashAppEntry>,
    extra_esptool_args: ExtraEsptoolArgs,
}

#[derive(Debug, Deserialize)]
struct FlashAppEntry {
    offset: String,
    file: String,
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

fn format_flash_arg(offset: &str, file: &str, windows_dir: &str) -> String {
    let normalized = format!("build\\{}", file).replace('/', "\\");
    format!("{offset} {windows_dir}\\{normalized}")
}

fn load_flasher_args_json() -> Result<(FlasherArgsJson, String), Box<dyn std::error::Error>> {
    let project_dir = project_dir()?;
    let windows_dir = wsl_to_windows_path(&project_dir)?;
    let json_path = project_dir.join("build/flasher_args.json");
    let root: FlasherArgsJson = serde_json::from_str(&fs::read_to_string(&json_path)?)?;
    Ok((root, windows_dir))
}

fn load_flasher_build_info() -> Result<FlasherBuildInfo, Box<dyn std::error::Error>> {
    let (root, windows_dir) = load_flasher_args_json()?;

    let flash_args = root
        .flash_files
        .iter()
        .map(|(offset, file_path)| format_flash_arg(offset, file_path, &windows_dir))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(FlasherBuildInfo {
        chip: root.extra_esptool_args.chip,
        flash_args,
    })
}

fn load_app_flash_info() -> Result<FlasherBuildInfo, Box<dyn std::error::Error>> {
    let (root, windows_dir) = load_flasher_args_json()?;
    let app = root
        .app
        .as_ref()
        .ok_or("flasher_args.json 中缺少 app 字段")?;

    Ok(FlasherBuildInfo {
        chip: root.extra_esptool_args.chip,
        flash_args: format_flash_arg(&app.offset, &app.file, &windows_dir),
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

fn idf_build() -> Result<(), Box<dyn std::error::Error>> {
    let status = Command::new("idf.py")
        .arg("build")
        .current_dir(project_dir()?)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    check_exit(status, "idf.py build")
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

fn require_esptool() -> Result<String, Box<dyn std::error::Error>> {
    env::var("ESPTOOL").map_err(|_| "未设置 ESPTOOL 环境变量（Windows 侧 esptool 路径）".into())
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
    #[arg(long, help = "关于")]
    about: bool,

    #[arg(short, long, help = "设备端口号")]
    port: Option<String>,

    #[arg(
        short,
        long,
        value_enum,
        help = "flash(烧录) flash-app(仅烧录app) flash-erase(擦除后烧录) erase(擦除) merge(合并) extract-bins(提取分区bin)"
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
    FlashErase,
    /// 仅烧录 app 分区固件
    FlashApp,
    /// 提取各分区 bin 到 flash-bins/ 并附加烧录地址后缀
    ExtractBins,
}

fn flash(esptool_path: &str, port: &str, erase: bool) -> Result<(), Box<dyn std::error::Error>> {
    idf_build()?;
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

fn flash_app(esptool_path: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    idf_build()?;
    let info = load_app_flash_info()?;
    run_powershell(&format!(
        "{esptool_path} -p {port} -c {} -b 1152000 --before default-reset --after hard-reset write-flash --flash-mode dio {}",
        info.chip, info.flash_args
    ))
}

fn erase(esptool_path: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    run_powershell(&format!(
        "{esptool_path} -p {port} -b 1152000 erase-flash"
    ))
}

fn parse_flash_offset(offset: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let trimmed = offset.trim();
    u64::from_str_radix(
        trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X"))
            .unwrap_or(trimmed),
        16,
    )
    .map_err(|_| format!("无效的烧录地址: {offset}").into())
}

fn flash_bin_dest_name(file_path: &str, offset: &str) -> String {
    let stem = Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("flash");
    format!("{stem}_{offset}.bin")
}

fn extract_bins() -> Result<(), Box<dyn std::error::Error>> {
    let project_dir = project_dir()?;
    let (root, _) = load_flasher_args_json()?;
    let out_dir = project_dir.join("flash-bins");

    if out_dir.exists() {
        fs::remove_dir_all(&out_dir)?;
    }
    fs::create_dir_all(&out_dir)?;

    let mut entries: Vec<_> = root.flash_files.iter().collect();
    entries.sort_by_key(|(offset, _)| parse_flash_offset(offset).unwrap_or(u64::MAX));

    if entries.is_empty() {
        return Err("flasher_args.json 中 flash_files 为空".into());
    }

    for (offset, file_path) in entries {
        let src = project_dir.join("build").join(file_path);
        if !src.exists() {
            return Err(format!(
                "源文件不存在: {}，请先执行 idf.py build",
                src.display()
            )
            .into());
        }

        let dest_name = flash_bin_dest_name(file_path, offset);
        let dest = out_dir.join(&dest_name);
        fs::copy(&src, &dest)?;
        println!("{} -> flash-bins/{}", src.display(), dest_name);
    }

    println!(
        "已提取 {} 个 bin 到 {}/",
        root.flash_files.len(),
        out_dir.display()
    );
    Ok(())
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
    let args = Args::parse();

    if args.about {
        branding::print_about();
        return Ok(());
    }

    port_list()?;

    let monitor_baud = parse_monitor_trailing(&args.trailing)?;

    match args.command {
        Some(EsptoolCommand::ExtractBins) => extract_bins()?,
        Some(EsptoolCommand::Flash) => {
            let esptool_path = require_esptool()?;
            flash(&esptool_path, require_port(&args.port)?, false)?;
        }
        Some(EsptoolCommand::Erase) => {
            let esptool_path = require_esptool()?;
            erase(&esptool_path, require_port(&args.port)?)?;
        }
        Some(EsptoolCommand::Merge) => {
            let esptool_path = require_esptool()?;
            merge(&esptool_path)?;
        }
        Some(EsptoolCommand::FlashErase) => {
            let esptool_path = require_esptool()?;
            flash(&esptool_path, require_port(&args.port)?, true)?;
        }
        Some(EsptoolCommand::FlashApp) => {
            let esptool_path = require_esptool()?;
            flash_app(&esptool_path, require_port(&args.port)?)?;
        }
        None => {}
    }

    if let Some(baud) = monitor_baud {
        monitor(require_port(&args.port)?, baud)?;
    }

    Ok(())
}
