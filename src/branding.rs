//! Copyright and author branding for wsl-idf.

pub const AUTHOR: &str = "wheat";
pub const EMAIL: &str = "vaemc520@qq.com";
pub const COPYRIGHT: &str = "Copyright (c) 2025-2026 wheat. All rights reserved.";
pub const REPO: &str = "https://github.com/vaemc/wsl-idf";

const WHEAT_ASCII: &str = r#"
 __        __   _   _    _____       _       _____
 \ \      / /  | | | |  | ____|     / \     |_   _|
  \ \ /\ / /   | |_| |  |  _|      / _ \      | |
   \ V  V /    |  _  |  | |___    / ___ \     | |
    \_/\_/     |_| |_|  |_____|  /_/   \_\    |_|
"#;

pub fn print_about() {
    println!("\x1b[33m{WHEAT_ASCII}\x1b[0m");
    println!("wsl-idf — ESP-IDF flash & monitor from WSL2");
    println!();
    println!("Author:  {AUTHOR} <{EMAIL}>");
    println!("{COPYRIGHT}");
    println!("Repo:    {REPO}");
}
