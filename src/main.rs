use clap::Parser;
use serde_json::{Value, json};
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

pub fn get_flasher_args() -> Result<Value, Box<dyn std::error::Error>> {
    let windows_path =
        wsl_to_windows_path(env::current_dir().unwrap().display().to_string().as_str())?;

    let flasher_args = &format!(
        "{}/build/flasher_args.json",
        env::current_dir().unwrap().display()
    );
    let json_path = Path::new(flasher_args);

    let file = File::open(json_path)?;
    let root_json: Value = serde_json::from_reader(file)?;

    let chip = root_json["extra_esptool_args"]["chip"]
        .as_str() // 转换为字符串（返回Option<&str>）
        .ok_or("未找到chip字段或字段类型不是字符串")?; // 处理空值

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
    Ok(json!({
        "flasher_args": final_flash_str,
        "chip": chip
    }))
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
            Err(_e) => {
                print!("{:?}", &buffer[0..bytes_read]);
            }
        }
    }
    cmd.wait().unwrap();
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, help = "设备端口号")]
    port: Option<String>,

    #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new( ["flash", "erase", "merge", "flashx"]), help = "flash(烧录) erase(擦除) merge(合并) flashx(擦除后烧录)")]
    command: Option<String>,
}

fn flash(esptool_path: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json_value = get_flasher_args()?;
    let full_args = format!(
        "{} -p {} -c {} -b 1152000 --before default-reset --after hard-reset write-flash --flash-mode dio {}",
        esptool_path,
        port,
        json_value["chip"].as_str().unwrap(),
        json_value["flasher_args"].as_str().unwrap()
    );
    run_shell_command("powershell.exe", &["-Command", &full_args.to_string()]);

    Ok(())
}

fn erase(esptool_path: &str, port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let full_args = format!("{} -p {} -b 1152000 erase-flash", esptool_path, port);
    run_shell_command("powershell.exe", &["-Command", &full_args.to_string()]);
    Ok(())
}

fn merge(esptool_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let current_dir = env::current_dir().unwrap();
    let json_value = get_flasher_args()?;
    let full_args = format!(
        "{} -c {} merge-bin -o {}/full-{}.bin {}",
        esptool_path,
        json_value["chip"].as_str().unwrap(),
        current_dir.display().to_string().as_str(),
        current_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap(),
        json_value["flasher_args"].as_str().unwrap()
    );
    run_shell_command("powershell.exe", &["-Command", &full_args.to_string()]);
    Ok(())
}

fn port_list() -> Result<(), Box<dyn std::error::Error>> {
    run_shell_command(
        "powershell.exe",
        &[
            "-Command",
            "Get-WmiObject -Class Win32_SerialPort | Select-Object -ExpandProperty DeviceID",
        ],
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    port_list()?;
    let esptool_path = env::var("ESPTOOL").expect("ESPTOOL WINDOWS路径环境变量未设置");
    let args = Args::parse();
    match args.command {
        Some(command) => match command.to_string().as_str() {
            "flash" => flash(&esptool_path, &args.port.unwrap())?,
            "erase" => erase(&esptool_path, &args.port.unwrap())?,
            "merge" => merge(&esptool_path)?,
            "flashx" => {
                erase(&esptool_path, &args.port.clone().unwrap())?;
                flash(&esptool_path, &args.port.unwrap())?
            }
            &_ => {
                eprintln!("没有这个命令");
            }
        },
        None => {}
    }
    Ok(())
}
