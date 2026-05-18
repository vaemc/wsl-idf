use super::{EspLogLevel, message_body};
use std::borrow::Cow;

const PURPLE: &str = "\x1b[35m";
const TRIPLE_AT_MARKER: &str = "@@@";

type ColorRule = fn(&str, EspLogLevel) -> Option<&'static str>;

static RULES: &[ColorRule] = &[info_body_starts_with_triple_at];

/// 在默认等级色之上应用特殊规则；无规则命中则返回等级默认色。
pub fn resolve_ansi_color(line: &str, level: EspLogLevel) -> &'static str {
    for rule in RULES {
        if let Some(color) = rule(line, level) {
            return color;
        }
    }
    level.ansi_prefix()
}

/// 输出行文本：命中 `@@@` 规则时去掉正文前的标记，不修改原串口缓冲。
pub fn display_line<'a>(line: &'a str, level: EspLogLevel) -> Cow<'a, str> {
    let Some(body) = message_body(line) else {
        return Cow::Borrowed(line);
    };
    let Some(stripped) = info_body_without_marker(body, level) else {
        return Cow::Borrowed(line);
    };
    let prefix_len = line.len() - body.len();
    Cow::Owned(format!("{}{}", &line[..prefix_len], stripped))
}

/// I 级且 TAG 后正文以 `@@@` 开头 → 紫色。
fn info_body_starts_with_triple_at(line: &str, level: EspLogLevel) -> Option<&'static str> {
    message_body(line)
        .and_then(|body| info_body_without_marker(body, level))
        .map(|_| PURPLE)
}

fn info_body_without_marker(body: &str, level: EspLogLevel) -> Option<&str> {
    if level != EspLogLevel::Info {
        return None;
    }
    body.strip_prefix(TRIPLE_AT_MARKER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serial_output::{detect_esp_idf_level, esp_idf_ansi_color};

    #[test]
    fn info_triple_at_is_purple() {
        let line = "I (1000) LOG_DEMO: @@@自定义高亮";
        assert_eq!(detect_esp_idf_level(line), Some(EspLogLevel::Info));
        assert_eq!(esp_idf_ansi_color(line), Some("\x1b[35m"));
    }

    #[test]
    fn info_triple_at_strips_marker_from_display() {
        let line = "I (1000) LOG_DEMO: @@@自定义高亮";
        let level = detect_esp_idf_level(line).unwrap();
        assert_eq!(
            display_line(line, level).as_ref(),
            "I (1000) LOG_DEMO: 自定义高亮"
        );
    }

    #[test]
    fn normal_info_stays_green() {
        let line = "I (1000) LOG_DEMO: [INFO] 系统运行正常";
        assert_eq!(esp_idf_ansi_color(line), Some("\x1b[32m"));
    }
}
