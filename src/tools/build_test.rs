use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

// --- Verify Boot Artifacts ---

pub struct VerifyBootArtifacts;

#[async_trait]
impl ToolHandler for VerifyBootArtifacts {
    fn info(&self) -> Tool {
        Tool {
            name: "verify_boot_artifacts".into(),
            description: "Verify RPi5 boot artifacts: kernel8.img, initramfs, DTB, config.txt, cmdline.txt".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["list", "check", "cmdline"], "default": "check" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("check");

        match action {
            "list" => {
                let result = crate::command::run("ls", &["-lh", "/boot"]).await?;
                Ok(serde_json::json!({ "boot_dir": result.stdout }))
            }
            "check" => {
                let required = ["kernel8.img", "initramfs-linux.img", "bcm2712-rpi-5-b.dtb", "config.txt", "cmdline.txt"];
                let optional = ["initramfs-linux-fallback.img", "boot-orders", "overlays"];
                let boot = std::path::Path::new("/boot");
                let mut results = Vec::new();
                for f in &required {
                    results.push(serde_json::json!({ "file": f, "exists": boot.join(f).exists(), "required": true }));
                }
                for f in &optional {
                    results.push(serde_json::json!({ "file": f, "exists": boot.join(f).exists(), "required": false }));
                }
                Ok(serde_json::json!({ "boot_path": "/boot", "files": results }))
            }
            "cmdline" => {
                let content = std::fs::read_to_string("/boot/cmdline.txt").unwrap_or_default();
                Ok(serde_json::json!({ "cmdline": content.trim() }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Verify Service Health ---

pub struct VerifyServiceHealth;

#[async_trait]
impl ToolHandler for VerifyServiceHealth {
    fn info(&self) -> Tool {
        Tool {
            name: "verify_service_health".into(),
            description: "Verify systemd service health: status, checklist (checklist of critical services), compare (against a provided checklist)".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["status", "checklist", "compare"] },
                    "checklist": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of service names for compare mode"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "status" => {
                let result = crate::command::run("systemctl", &["--failed"]).await?;
                let failed = result.stdout.lines().filter(|l| !l.is_empty()).count();
                let all = crate::command::run("systemctl", &["list-units", "--type=service", "--state=running", "--no-pager"]).await?;
                let running = all.stdout.lines().filter(|l| !l.is_empty()).count();
                Ok(serde_json::json!({ "failed": failed.saturating_sub(1), "running": running.saturating_sub(1), "failed_list": result.stdout }))
            }
            "checklist" => {
                let critical = &["sshd", "NetworkManager", "systemd-journald", "systemd-udevd", "systemd-logind", "systemd-resolved", "pacman-filesdb"];
                let mut results = Vec::new();
                for svc in critical {
                    let r = crate::command::run("systemctl", &["is-active", svc]).await;
                    results.push(serde_json::json!({ "service": svc, "active": r.map(|o| o.stdout.trim() == "active").unwrap_or(false) }));
                }
                Ok(serde_json::json!({ "checklist": results }))
            }
            "compare" => {
                let checklist = args.get("checklist").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                let mut results = Vec::new();
                for item in &checklist {
                    let name = item.as_str().unwrap_or("unknown");
                    let r = crate::command::run("systemctl", &["is-active", name]).await;
                    results.push(serde_json::json!({ "service": name, "active": r.map(|o| o.stdout.trim() == "active").unwrap_or(false) }));
                }
                Ok(serde_json::json!({ "compare": results }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Verify Homectl User ---

pub struct VerifyHomectlUser;

#[async_trait]
impl ToolHandler for VerifyHomectlUser {
    fn info(&self) -> Tool {
        Tool {
            name: "verify_homectl_user".into(),
            description: "Verify systemd-homed user configuration and status".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["status", "check"], "default": "status" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("status");

        match action {
            "status" => {
                let result = crate::command::run("homectl", &["list"]).await;
                let (users, available) = match result {
                    Ok(r) => (r.stdout, true),
                    Err(_) => (String::new(), false),
                };
                Ok(serde_json::json!({ "users": users, "homed_available": available }))
            }
            "check" => {
                let service = crate::command::run("systemctl", &["is-active", "systemd-homed"]).await;
                Ok(serde_json::json!({ "systemd-homed": service.map(|r| r.stdout.trim().to_string()).unwrap_or_default() }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Compare Fstab ---

pub struct CompareFstab;

#[async_trait]
impl ToolHandler for CompareFstab {
    fn info(&self) -> Tool {
        Tool {
            name: "compare_fstab".into(),
            description: "Compare fstab against expected RPi5 BTRFS layout: subvolumes and mount options".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["check", "options"], "default": "check" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let fstab = std::fs::read_to_string("/etc/fstab").unwrap_or_default();
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("check");

        match action {
            "check" => {
                let expected_subvols = ["@", "@home", "@var", "@cache", "@log", "@tmp", "@snapshots", "@swap"];
                let mut results = Vec::new();
                for sv in &expected_subvols {
                    let found = fstab.lines().any(|l| l.contains(&format!("subvol=/{}", sv)));
                    results.push(serde_json::json!({ "subvolume": sv, "found": found }));
                }
                Ok(serde_json::json!({ "fstab_entries": fstab.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).count(), "subvolumes": results }))
            }
            "options" => {
                let mount_opts: Vec<&str> = fstab.lines().filter_map(|l| {
                    if l.starts_with('#') || l.trim().is_empty() { return None; }
                    l.split_whitespace().nth(3)
                }).collect();
                Ok(serde_json::json!({ "mount_options": mount_opts }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Compare Packages ---

pub struct ComparePackages;

#[async_trait]
impl ToolHandler for ComparePackages {
    fn info(&self) -> Tool {
        Tool {
            name: "compare_packages".into(),
            description: "Compare installed packages against build configuration: diff, missing, extra".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["diff", "missing", "extra"], "default": "diff" },
                    "build_conf_path": { "type": "string", "default": "/etc/build.conf", "description": "Path to package list config" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("diff");
        let conf_path = args.get("build_conf_path").and_then(|v| v.as_str()).unwrap_or("/etc/build.conf");

        let conf_content = std::fs::read_to_string(conf_path).unwrap_or_default();
        let expected: Vec<String> = conf_content.lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .map(|l| l.trim().to_string())
            .collect();

        let installed_result = crate::command::run("pacman", &["-Qq"]).await?;
        let installed: Vec<&str> = installed_result.stdout.lines().collect();

        match action {
            "diff" => {
                let missing: Vec<&str> = expected.iter().filter(|p| !installed.contains(&p.as_str())).map(|s| s.as_str()).collect();
                let extra: Vec<&str> = installed.iter().filter(|p| !expected.contains(&p.to_string())).copied().collect();
                Ok(serde_json::json!({ "expected_count": expected.len(), "installed_count": installed.len(), "missing": missing, "extra": extra }))
            }
            "missing" => {
                let missing: Vec<&str> = expected.iter().filter(|p| !installed.contains(&p.as_str())).map(|s| s.as_str()).collect();
                Ok(serde_json::json!({ "missing": missing, "count": missing.len() }))
            }
            "extra" => {
                let extra: Vec<&str> = installed.iter().filter(|p| !expected.contains(&p.to_string())).copied().collect();
                Ok(serde_json::json!({ "extra": extra, "count": extra.len() }))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

// --- Check Security Posture ---

pub struct CheckSecurityPosture;

#[async_trait]
impl ToolHandler for CheckSecurityPosture {
    fn info(&self) -> Tool {
        Tool {
            name: "check_security_posture".into(),
            description: "Check SSH, fail2ban, sudo, MCP security posture".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["full", "sshd", "fail2ban", "sudo", "mcp"], "default": "full" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("full");
        let mut results = serde_json::Map::new();

        if action == "full" || action == "sshd" {
            let permit_root = std::fs::read_to_string("/etc/ssh/sshd_config")
                .map(|c| c.lines().find(|l| l.contains("PermitRootLogin")).map(|l| l.to_string()))
                .ok().flatten();
            let password_auth = std::fs::read_to_string("/etc/ssh/sshd_config")
                .map(|c| c.lines().find(|l| l.contains("PasswordAuthentication")).map(|l| l.to_string()))
                .ok().flatten();
            results.insert("sshd_permit_root_login".into(), Value::String(permit_root.unwrap_or("not found".into())));
            results.insert("sshd_password_auth".into(), Value::String(password_auth.unwrap_or("not found".into())));
        }

        if action == "full" || action == "fail2ban" {
            let active = crate::command::run("systemctl", &["is-active", "fail2ban"]).await;
            results.insert("fail2ban_active".into(), Value::String(active.map(|r| r.stdout.trim().into()).unwrap_or("not found".into())));
            let jails = crate::command::run("fail2ban-client", &["status"]).await;
            results.insert("fail2ban_jails".into(), Value::String(jails.map(|r| r.stdout).unwrap_or_default()));
        }

        if action == "full" || action == "sudo" {
            let sudoers = std::fs::read_to_string("/etc/sudoers").unwrap_or_default();
            let wheel_nopass = sudoers.lines().any(|l| l.contains("%wheel") && l.contains("NOPASSWD"));
            results.insert("sudo_wheel_nopasswd".into(), Value::Bool(wheel_nopass));
        }

        Ok(Value::Object(results))
    }
}

// --- Check RPi Hardware ---

pub struct CheckRpiHardware;

#[async_trait]
impl ToolHandler for CheckRpiHardware {
    fn info(&self) -> Tool {
        Tool {
            name: "check_rpi_hardware".into(),
            description: "Check RPi5 hardware via vcgencmd: full, eeprom, temperature, frequencies, voltage, memory".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["full", "eeprom", "temperature", "frequencies", "voltage", "memory"], "default": "full" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("full");
        let mut data = serde_json::Map::new();

        let is_full = action == "full";

        if is_full || action == "temperature" {
            if let Ok(r) = crate::command::run("vcgencmd", &["measure_temp"]).await {
                data.insert("temp".into(), Value::String(r.stdout.trim().into()));
            }
        }
        if is_full || action == "frequencies" {
            for (cmd, label) in &[("measure_clock arm", "arm_freq"), ("measure_clock core", "core_freq"), ("measure_clock h264", "h264_freq"), ("measure_clock v3d", "v3d_freq")] {
                if let Ok(r) = crate::command::run("vcgencmd", &[cmd]).await {
                    data.insert(label.to_string(), Value::String(r.stdout.trim().into()));
                }
            }
        }
        if is_full || action == "voltage" {
            for (cmd, label) in &[("measure_volts core", "core_voltage"), ("measure_volts sdram_c", "sdram_c_voltage"), ("measure_volts sdram_i", "sdram_i_voltage")] {
                if let Ok(r) = crate::command::run("vcgencmd", &[cmd]).await {
                    data.insert(label.to_string(), Value::String(r.stdout.trim().into()));
                }
            }
        }
        if is_full || action == "memory" {
            if let Ok(r) = crate::command::run("vcgencmd", &["get_mem arm"]).await {
                data.insert("arm_mem".into(), Value::String(r.stdout.trim().into()));
            }
            if let Ok(r) = crate::command::run("vcgencmd", &["get_mem gpu"]).await {
                data.insert("gpu_mem".into(), Value::String(r.stdout.trim().into()));
            }
        }
        if action == "eeprom" {
            if let Ok(r) = crate::command::run("vcgencmd", &["bootloader_version"]).await {
                data.insert("bootloader".into(), Value::String(r.stdout.trim().into()));
            }
            if let Ok(r) = crate::command::run("rpi-eeprom-config", &[]).await {
                let order = r.stdout.lines().find(|l| l.starts_with("BOOT_ORDER"));
                data.insert("boot_order".into(), Value::String(order.unwrap_or("not found").into()));
            }
        }

        Ok(Value::Object(data))
    }
}

// --- Benchmark Quick ---

pub struct BenchmarkQuick;

#[async_trait]
impl ToolHandler for BenchmarkQuick {
    fn info(&self) -> Tool {
        Tool {
            name: "benchmark_quick".into(),
            description: "Run quick benchmarks: full, disk, cpu, memory, network".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["full", "disk", "cpu", "memory", "network"], "default": "full" }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() { return Err(ToolError::platform_not_arch()); }

        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("full");
        let mut results = serde_json::Map::new();

        if action == "full" || action == "disk" {
            let tmp = "/tmp/.arch-bench";
            let start = std::time::Instant::now();
            let _ = crate::command::run("dd", &["if=/dev/zero", &format!("of={}", tmp), "bs=1M", "count=100", "oflag=direct"]).await;
            let elapsed = start.elapsed();
            let _ = std::fs::remove_file(tmp);
            let mb_per_sec = if elapsed.as_secs_f64() > 0.0 { 100.0 / elapsed.as_secs_f64() } else { 0.0 };
            results.insert("disk_write_mb_s".into(), Value::Number(serde_json::Number::from_f64(mb_per_sec).unwrap_or(serde_json::Number::from(0))));
        }

        if action == "full" || action == "cpu" {
            let start = std::time::Instant::now();
            let _result: u64 = (0..1_000_000).map(|i| i * i).sum();
            let elapsed = start.elapsed();
            results.insert("cpu_ops_per_sec".into(), Value::Number(serde_json::Number::from_f64(1_000_000.0 / elapsed.as_secs_f64()).unwrap_or(serde_json::Number::from(0))));
        }

        if action == "full" || action == "memory" {
            let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
            let total = parse_mem(&meminfo, "MemTotal");
            let avail = parse_mem(&meminfo, "MemAvailable");
            results.insert("mem_total_kb".into(), Value::Number(serde_json::Number::from(total)));
            results.insert("mem_avail_kb".into(), Value::Number(serde_json::Number::from(avail)));
            results.insert("mem_used_pct".into(), Value::Number(serde_json::Number::from_f64(if total > 0 { (total - avail) as f64 / total as f64 * 100.0 } else { 0.0 }).unwrap_or(serde_json::Number::from(0))));
        }

        if action == "full" || action == "network" {
            let start = std::time::Instant::now();
            let _ = crate::client::head_latency("https://archlinux.org/").await;
            let latency = start.elapsed().as_millis() as u64;
            results.insert("archlinux_org_latency_ms".into(), Value::Number(serde_json::Number::from(latency)));
        }

        Ok(Value::Object(results))
    }
}

fn parse_mem(content: &str, key: &str) -> u64 {
    content.lines().find(|l| l.starts_with(key))
        .and_then(|l| l.split(':').nth(1))
        .and_then(|v| v.trim().split_whitespace().next())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
