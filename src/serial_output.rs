use std::io::{self, Read, Write};

/// ESP-IDF 日志等级（行首单字符 E / W / I / D / V）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EspLogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Verbose,
}

impl EspLogLevel {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'E' => Some(Self::Error),
            'W' => Some(Self::Warn),
            'I' => Some(Self::Info),
            'D' => Some(Self::Debug),
            'V' => Some(Self::Verbose),
            _ => None,
        }
    }

    /// ANSI 前景色（与 idf.py monitor 常见配色接近）。
    pub fn ansi_prefix(self) -> &'static str {
        match self {
            Self::Error => "\x1b[31m",   // 红
            Self::Warn => "\x1b[33m",    // 黄
            Self::Info => "\x1b[32m",    // 绿
            Self::Debug => "\x1b[36m",   // 青
            Self::Verbose => "\x1b[90m", // 灰
        }
    }
}

/// 识别 ESP-IDF 日志行：`I (640797) LOG_DEMO: ...`
pub fn detect_esp_idf_level(line: &str) -> Option<EspLogLevel> {
    let level = detect_level_char(line)?;
    EspLogLevel::from_char(level)
}

/// 命中 ESP-IDF 格式时返回对应 ANSI 颜色前缀。
pub fn esp_idf_ansi_color(line: &str) -> Option<&'static str> {
    detect_esp_idf_level(line).map(EspLogLevel::ansi_prefix)
}

fn detect_level_char(line: &str) -> Option<char> {
    level_from_prefix(line).or_else(|| level_from_tokens(line))
}

fn level_from_prefix(line: &str) -> Option<char> {
    let mut chars = line.chars();
    let level = chars.next()?;
    if chars.next()? != ' ' || chars.next()? != '(' {
        return None;
    }
    let mut digits = 0usize;
    for c in chars {
        if c.is_ascii_digit() {
            digits += 1;
        } else if c == ')' && digits > 0 {
            return Some(level);
        } else {
            return None;
        }
    }
    None
}

fn level_from_tokens(line: &str) -> Option<char> {
    let mut parts = line.split_whitespace();
    let level = parts.next()?;
    let timestamp = parts.next()?;
    if level.len() != 1 {
        return None;
    }
    let level_ch = level.chars().next()?;
    let inner = timestamp.strip_prefix('(')?.strip_suffix(')')?;
    if inner.is_empty() || !inner.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(level_ch)
}

/// 将一行串口文本写入输出流（ESP-IDF 行自动着色）。
pub fn write_line(out: &mut impl Write, line: &[u8]) -> io::Result<()> {
    if let Ok(text) = std::str::from_utf8(line) {
        if let Some(color) = esp_idf_ansi_color(text) {
            out.write_all(color.as_bytes())?;
            out.write_all(line)?;
            out.write_all(b"\x1b[0m\n")?;
            return Ok(());
        }
    }
    out.write_all(line)?;
    out.write_all(b"\n")?;
    Ok(())
}

/// 将新读到的字节追加到缓冲，按行处理后写出。
pub fn process_chunk(
    pending: &mut Vec<u8>,
    chunk: &[u8],
    out: &mut impl Write,
) -> io::Result<()> {
    pending.extend_from_slice(chunk);
    while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
        let mut line: Vec<u8> = pending.drain(..=pos).collect();
        line.pop();
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        write_line(out, &line)?;
    }
    Ok(())
}

/// 从任意 Read 源按行读取并着色输出到 stdout。
pub fn stream_colored<R: Read>(mut reader: R) -> io::Result<()> {
    let mut read_buf = [0u8; 1024];
    let mut pending = Vec::new();
    let mut out = io::stdout().lock();

    loop {
        let n = reader.read(&mut read_buf)?;
        if n == 0 {
            if !pending.is_empty() {
                write_line(&mut out, &pending)?;
            }
            break;
        }
        process_chunk(&mut pending, &read_buf[..n], &mut out)?;
    }

    Ok(())
}

/// 从子进程 stdout 读取串口监视数据并着色输出。
pub fn stream_child_stdout(child: &mut std::process::Child) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = child
        .stdout
        .take()
        .ok_or("无法获取子进程标准输出")?;

    stream_colored(io::BufReader::new(stdout))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_esp_idf_levels() {
        let line = "E (640797) LOG_DEMO: [ERROR] 这是一条错误日志";
        assert_eq!(detect_esp_idf_level(line), Some(EspLogLevel::Error));
        assert_eq!(
            detect_esp_idf_level("I (637787) LOG_DEMO: === 本轮打印结束 ==="),
            Some(EspLogLevel::Info)
        );
        assert_eq!(
            detect_esp_idf_level("W (643817) LOG_DEMO: [WARN] 内存使用率接近上限"),
            Some(EspLogLevel::Warn)
        );
        assert_eq!(
            detect_esp_idf_level("D (1000) wifi: station connect"),
            Some(EspLogLevel::Debug)
        );
        assert_eq!(
            detect_esp_idf_level("V (1000) heap: free 12345"),
            Some(EspLogLevel::Verbose)
        );
    }

    #[test]
    fn ignores_non_log_lines() {
        assert_eq!(detect_esp_idf_level("COM20 已打开，波特率：115200"), None);
        assert_eq!(detect_esp_idf_level("X (123) TAG: msg"), None);
    }
}
