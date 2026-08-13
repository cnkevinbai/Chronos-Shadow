// 本地开发环境检测器 + 自动安装引擎
// 检测 Python/Node/Git + 关键包, 支持一键安装缺失依赖

use serde::{Deserialize, Serialize};
use std::process::Command;

/// 完整环境剖面 — 实际应用环境感知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentProfile {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub user: String,
    pub home_dir: String,
    pub temp_dir: String,
    pub current_dir: String,
    pub cpu_cores: usize,
    pub disk_free_gb: f64,
    pub tools: Vec<ToolStatus>,
}

/// 获取完整环境剖面
pub fn get_environment_profile() -> EnvironmentProfile {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let temp = std::env::temp_dir().to_string_lossy().to_string();
    let current = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();

    EnvironmentProfile {
        os,
        arch,
        hostname: hostname(),
        user: std::env::var("USERNAME").unwrap_or_else(|_| "unknown".into()),
        home_dir: home,
        temp_dir: temp,
        current_dir: current,
        cpu_cores: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
        disk_free_gb: get_disk_free_gb(),
        tools: vec![
            check_python(), check_node(), check_git(), check_python_pptx(), check_cargo(),
        ],
    }
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".into())
}

fn get_disk_free_gb() -> f64 {
    #[cfg(target_os = "windows")]
    {
        if let Ok(out) = Command::new("wmic").args(["LogicalDisk", "where", "DeviceID='C:'", "get", "FreeSpace"]).output() {
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(line) = s.lines().nth(1) {
                if let Ok(bytes) = line.trim().parse::<u64>() {
                    return bytes as f64 / 1024.0 / 1024.0 / 1024.0;
                }
            }
        }
    }
    0.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
    pub install_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvReport {
    pub tools: Vec<ToolStatus>,
    pub ready: bool,
    pub missing_count: usize,
    pub recommendations: Vec<String>,
}

/// 检测所有开发环境工具
pub fn check_environment() -> EnvReport {
    let tools = vec![
        check_python(),
        check_node(),
        check_git(),
        check_python_pptx(),
        check_cargo(),
    ];

    let missing: Vec<_> = tools.iter().filter(|t| !t.installed).collect();
    let recommendations: Vec<String> = missing.iter().map(|t| t.install_hint.clone()).collect();

    EnvReport {
        ready: missing.is_empty(),
        missing_count: missing.len(),
        tools,
        recommendations,
    }
}

fn check_python() -> ToolStatus {
    match Command::new("python").arg("--version").output() {
        Ok(out) => {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            ToolStatus { name: "Python".into(), installed: true, version: Some(v), path: which("python"),
                install_hint: "https://python.org 下载安装".into() }
        }
        Err(_) => ToolStatus { name: "Python".into(), installed: false, version: None, path: None,
            install_hint: "winget install python3 或访问 python.org".into() },
    }
}

fn check_node() -> ToolStatus {
    match Command::new("node").arg("--version").output() {
        Ok(out) => {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            ToolStatus { name: "Node.js".into(), installed: true, version: Some(v), path: which("node"),
                install_hint: "https://nodejs.org 下载安装".into() }
        }
        Err(_) => ToolStatus { name: "Node.js".into(), installed: false, version: None, path: None,
            install_hint: "winget install OpenJS.NodeJS".into() },
    }
}

fn check_git() -> ToolStatus {
    match Command::new("git").arg("--version").output() {
        Ok(out) => {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            ToolStatus { name: "Git".into(), installed: true, version: Some(v), path: which("git"),
                install_hint: "https://git-scm.com 下载安装".into() }
        }
        Err(_) => ToolStatus { name: "Git".into(), installed: false, version: None, path: None,
            install_hint: "winget install Git.Git".into() },
    }
}

fn check_python_pptx() -> ToolStatus {
    match Command::new("python").args(["-c", "import pptx"]).output() {
        Ok(out) => {
            if out.status.success() {
                ToolStatus { name: "python-pptx (PPT生成)".into(), installed: true, version: None, path: None,
                    install_hint: "".into() }
            } else {
                ToolStatus { name: "python-pptx (PPT生成)".into(), installed: false, version: None, path: None,
                    install_hint: "pip install python-pptx".into() }
            }
        }
        Err(_) => ToolStatus { name: "python-pptx (PPT生成)".into(), installed: false, version: None, path: None,
            install_hint: "先安装 Python, 然后 pip install python-pptx".into() },
    }
}

fn check_cargo() -> ToolStatus {
    match Command::new("cargo").arg("--version").output() {
        Ok(out) => {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            ToolStatus { name: "Cargo/Rust".into(), installed: true, version: Some(v), path: which("cargo"),
                install_hint: "https://rustup.rs 安装".into() }
        }
        Err(_) => ToolStatus { name: "Cargo/Rust".into(), installed: false, version: None, path: None,
            install_hint: "winget install Rustlang.Rustup".into() },
    }
}

fn which(cmd: &str) -> Option<String> {
    Command::new("where").arg(cmd).output().ok()
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).to_string();
            s.lines().next().map(|l| l.trim().to_string())
        })
}

/// 自动安装缺失的依赖
pub fn auto_install_missing(report: &EnvReport) -> Vec<String> {
    let mut results = Vec::new();
    for tool in &report.tools {
        if tool.installed { continue; }
        let result = match tool.name.as_str() {
            "python-pptx (PPT生成)" => run_install("pip", &["install", "python-pptx"]),
            _ => format!("⏭ {} 需手动安装: {}", tool.name, tool.install_hint),
        };
        results.push(result);
    }
    results
}

fn run_install(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd).args(args).output() {
        Ok(out) => {
            if out.status.success() {
                format!("✅ {} {} 安装成功", cmd, args.join(" "))
            } else {
                let err = String::from_utf8_lossy(&out.stderr);
                format!("❌ {} 安装失败: {}", cmd, &err[..200.min(err.len())])
            }
        }
        Err(e) => format!("❌ 无法运行 {}: {}", cmd, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_check() {
        let report = check_environment();
        assert!(!report.tools.is_empty());
        // Python 和 Node.js 至少应该有一个
        let has_python = report.tools.iter().any(|t| t.name == "Python" && t.installed);
        let has_node = report.tools.iter().any(|t| t.name == "Node.js" && t.installed);
        assert!(has_python || has_node, "Should have at least one runtime");
    }
}
