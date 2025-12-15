use clap::Parser;
use serde_json::Value;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
pub fn wsl_to_windows_path(wsl_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("wslpath").arg("-w").arg(wsl_path).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("命令执行失败（退出码：{}）：{}", output.status, stderr).into());
    }
    let stdout = String::from_utf8(output.stdout)?;
    Ok(stdout)
}

pub fn get_flasher_args() -> Result<String, Box<dyn std::error::Error>> {
    let windows_path =
        wsl_to_windows_path(env::current_dir().unwrap().display().to_string().as_str())?;

    let flasher_args = &format!(
        "{}/build/flasher_args.json",
        env::current_dir().unwrap().display()
    );
    let json_path = Path::new(flasher_args);

    let file = File::open(json_path)?;
    let root_json: Value = serde_json::from_reader(file)?;

    // 3. 安全获取 flash_files 字段（检查类型和存在性）
    let flash_files = match root_json.get("flash_files") {
        Some(Value::Object(map)) => map,
        Some(_) => return Err("flash_files 字段类型错误（应为 JSON 对象）".into()),
        None => return Err("JSON 中未找到 flash_files 字段".into()),
    };

    // 4. 遍历 flash_files 键值对，拼接目标格式字符串（替换 / 为 \）
    let mut flash_str_parts = Vec::new();
    for (offset, file_path_value) in flash_files {
        // 确保文件路径是字符串类型
        let file_path = match file_path_value.as_str() {
            Some(s) => s,
            None => return Err(format!("偏移量 {} 对应的文件路径不是字符串类型", offset).into()),
        };
        // 核心：替换 / 为 \，并拼接 build\ 前缀
        let normalized_path = format!("build\\{}", file_path).replace('/', "\\");
        // 拼接 "偏移量 规范化路径" 格式的片段
        let part = format!("{} {}\\{}", offset, windows_path.trim(), normalized_path);
        flash_str_parts.push(part);
    }

    // 5. 合并所有片段为最终字符串（空格分隔）
    let final_flash_str = flash_str_parts.join(" ");

    Ok(final_flash_str)
}

fn run_shell_command(cmd: &str, args: &[&str]) {
    let mut cmd = Command::new(cmd)
        .args(args)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let stdout = cmd
        .stdout
        .as_mut()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "无法获取标准输出句柄"))
        .unwrap();

    let mut reader = io::BufReader::new(stdout);
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = reader.read(&mut buffer).unwrap();
        if bytes_read == 0 {
            break;
        }
        match std::str::from_utf8(&buffer[0..bytes_read]) {
            Ok(s) => print!("{}", s),
            Err(e) => {
                print!("{:?}", &buffer[0..bytes_read]);
            }
        }
    }
    cmd.wait().unwrap();
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    port: Option<String>,

    #[arg(short, long)]
    command: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        Some(command) => match command.to_string().as_str() {
            "flash" => {
                let flasher_args = get_flasher_args()?;
                let full_args = format!(
                    "D:\\esptool-windows-amd64\\esptool.exe -p {} -b 460800 --before default-reset --after hard-reset write-flash --flash-mode dio {}",
                    args.port.unwrap(),
                    flasher_args
                );
                run_shell_command("powershell.exe", &["-Command", &full_args.to_string()])
            }
            "erase" => {
                let full_args = format!(
                    "D:\\esptool-windows-amd64\\esptool.exe -p {} -b 460800 erase-flash",
                    args.port.unwrap()
                );
                run_shell_command("powershell.exe", &["-Command", &full_args.to_string()])
            }
            "merge" => {}
            &_ => {
                eprintln!("没有这个命令");
            }
        },

        None => {
            run_shell_command(
                "powershell.exe",
                &[
                    "-Command",
                    "Get-WmiObject -Class Win32_SerialPort | Select-Object -ExpandProperty DeviceID",
                ],
            );
        }
    }

    Ok(())
}
