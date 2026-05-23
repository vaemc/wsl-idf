use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const HEADER: &str = "\x1b[36m--- wsl-idf addr2line ---\x1b[0m";

/// 从 monitor 串口日志中检测 ESP 崩溃并调用 addr2line 解析地址。
pub struct CrashMonitor {
    ctx: Option<DecodeContext>,
    in_crash: bool,
    addresses: HashSet<String>,
}

struct DecodeContext {
    addr2line: PathBuf,
    elf: PathBuf,
}

impl CrashMonitor {
    pub fn new() -> Self {
        match DecodeContext::from_project() {
            Ok(ctx) => Self {
                ctx: Some(ctx),
                in_crash: false,
                addresses: HashSet::new(),
            },
            Err(reason) => {
                eprintln!(
                    "\x1b[33mwsl-idf: 崩溃地址自动解析未启用（{reason}）\x1b[0m"
                );
                Self {
                    ctx: None,
                    in_crash: false,
                    addresses: HashSet::new(),
                }
            }
        }
    }

    pub fn on_line(&mut self, line: &str, out: &mut impl Write) -> io::Result<()> {
        if self.ctx.is_none() {
            return Ok(());
        }

        if is_crash_start(line) {
            self.in_crash = true;
            self.addresses.clear();
        }

        if self.in_crash {
            for addr in extract_hex_addresses(line) {
                self.addresses.insert(addr);
            }
            if line.contains("Backtrace:") {
                for addr in extract_backtrace_pcs(line) {
                    self.addresses.insert(addr);
                }
            }
        }

        if self.in_crash && is_crash_end(line) {
            self.flush_decode(out)?;
            self.in_crash = false;
        }

        Ok(())
    }

    fn flush_decode(&mut self, out: &mut impl Write) -> io::Result<()> {
        let Some(ctx) = self.ctx.as_ref() else {
            return Ok(());
        };
        if self.addresses.is_empty() {
            return Ok(());
        }

        let mut addrs: Vec<String> = self.addresses.drain().collect();
        addrs.sort();

        let output = Command::new(&ctx.addr2line)
            .arg("-pfiaC")
            .arg("-e")
            .arg(&ctx.elf)
            .args(&addrs)
            .output()
            .map_err(|e| io::Error::other(format!("执行 {} 失败: {e}", ctx.addr2line.display())))?;

        writeln!(out)?;
        writeln!(out, "{HEADER}")?;
        if output.status.success() {
            out.write_all(&output.stdout)?;
        } else {
            out.write_all(&output.stdout)?;
            out.write_all(&output.stderr)?;
        }
        writeln!(out)?;
        Ok(())
    }
}

impl DecodeContext {
    fn from_project() -> Result<Self, String> {
        let project_dir = env::current_dir().map_err(|e| e.to_string())?;
        let build_dir = project_dir.join("build");
        let elf = find_elf(&project_dir, &build_dir).ok_or("未找到 build/*.elf")?;
        let addr2line = find_addr2line(&build_dir)
            .or_else(find_addr2line_on_path)
            .ok_or("未找到 *-addr2line（请先 source ESP-IDF export.sh）")?;

        if !elf.is_file() {
            return Err(format!("ELF 不存在: {}", elf.display()));
        }
        if !addr2line.is_file() {
            return Err(format!("addr2line 不存在: {}", addr2line.display()));
        }

        Ok(Self { addr2line, elf })
    }
}

fn find_elf(project_dir: &Path, build_dir: &Path) -> Option<PathBuf> {
    let desc_path = build_dir.join("project_description.json");
    if let Ok(content) = fs::read_to_string(&desc_path) {
        if let Ok(root) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(elf) = root.get("app_elf").and_then(|v| v.as_str()) {
                let path = project_dir.join(elf);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }

    let name = project_dir.file_name()?.to_str()?;
    let candidate = build_dir.join(format!("{name}.elf"));
    if candidate.is_file() {
        return Some(candidate);
    }

    fs::read_dir(build_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|ext| ext == "elf"))
}

fn find_addr2line(build_dir: &Path) -> Option<PathBuf> {
    let cache = fs::read_to_string(build_dir.join("CMakeCache.txt")).ok()?;
    for line in cache.lines() {
        if let Some(path) = line.strip_prefix("CMAKE_ADDR2LINE:FILEPATH=") {
            let path = PathBuf::from(path);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

fn find_addr2line_on_path() -> Option<PathBuf> {
    const CANDIDATES: &[&str] = &[
        "riscv32-esp-elf-addr2line",
        "xtensa-esp32s3-elf-addr2line",
        "xtensa-esp32s2-elf-addr2line",
        "xtensa-esp32-elf-addr2line",
        "xtensa-esp-elf-addr2line",
    ];
    for name in CANDIDATES {
        let output = Command::new("which").arg(name).output().ok()?;
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn is_crash_start(line: &str) -> bool {
    line.contains("Guru Meditation Error")
        || line.contains("register dump")
        || line.contains("Backtrace:")
        || line.contains("abort() was called")
        || line.contains("Stack memory:")
        || line.contains("Task watchdog got triggered")
        || line.contains("assert failed")
        || line.contains("Panic handler")
}

fn is_crash_end(line: &str) -> bool {
    line.contains("Please enable CONFIG_ESP_SYSTEM_USE_FRAME_POINTER")
        || line.contains("ELF file SHA256")
        || line.contains("Rebooting...")
        || line.contains("Backtrace stopped")
}

fn is_skipped_address(addr: &str) -> bool {
    matches!(
        addr,
        "0x00000000"
            | "0xdeadc0de"
            | "0xdeadbeef"
            | "0xabababab"
            | "0xcdcdcdcd"
    )
}

fn extract_hex_addresses(line: &str) -> Vec<String> {
    let mut addrs = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 2 < bytes.len() {
        if bytes[i] == b'0' && bytes[i + 1] == b'x' {
            let mut j = i + 2;
            while j < bytes.len() && bytes[j].is_ascii_hexdigit() {
                j += 1;
            }
            if j - i >= 10 {
                let addr = &line[i..j];
                if !is_skipped_address(addr) {
                    addrs.push(addr.to_string());
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    addrs
}

/// Backtrace 行中每对 `PC:SP` 只取 PC。
fn extract_backtrace_pcs(line: &str) -> Vec<String> {
    let Some(rest) = line.split("Backtrace:").nth(1) else {
        return Vec::new();
    };
    rest.split_whitespace()
        .filter_map(|token| {
            let pc = token.split(':').next()?;
            if pc.starts_with("0x") && !is_skipped_address(pc) {
                Some(pc.to_string())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_register_dump_addresses() {
        let line = "MEPC    : 0x480b4bb4  RA      : 0x480b3556  SP      : 0x4ff86920";
        let addrs = extract_hex_addresses(line);
        assert!(addrs.contains(&"0x480b4bb4".to_string()));
        assert!(addrs.contains(&"0x480b3556".to_string()));
        assert!(!addrs.contains(&"0xdeadc0de".to_string()));
    }

    #[test]
    fn skips_placeholder_addresses() {
        let line = "MSTATUS : 0x00001888  MCAUSE  : 0xdeadc0de  MTVAL   : 0xdeadc0de";
        let addrs = extract_hex_addresses(line);
        assert!(!addrs.iter().any(|a| a == "0xdeadc0de"));
    }

    #[test]
    fn extracts_backtrace_pc_only() {
        let line = "Backtrace: 0x4200748e:0x3fc9f980 0x42007592:0x3fc9f9a0";
        let addrs = extract_backtrace_pcs(line);
        assert_eq!(addrs, vec!["0x4200748e", "0x42007592"]);
    }

    #[test]
    fn detects_crash_markers() {
        assert!(is_crash_start(
            "E (52519) task_wdt: Task watchdog got triggered."
        ));
        assert!(is_crash_start("Core  0 register dump:"));
        assert!(is_crash_end(
            "Please enable CONFIG_ESP_SYSTEM_USE_FRAME_POINTER option to have a full backtrace."
        ));
    }
}
