use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::{Prompt, PromptArgument, PromptContent, PromptMessage};
use crate::tools::ToolHandler;
use serde_json::Value;
use std::collections::HashMap;

pub fn list() -> Vec<Prompt> {
    vec![
        Prompt {
            name: "troubleshoot_issue".into(),
            description: "Diagnose system errors and provide solutions using Arch Wiki knowledge".into(),
            arguments: vec![
                PromptArgument { name: "error_message".into(), description: "The error message or issue description".into(), required: true },
                PromptArgument { name: "context".into(), description: "Additional context about when/where the error occurred".into(), required: false },
            ],
        },
        Prompt {
            name: "audit_aur_package".into(),
            description: "Perform comprehensive security audit of an AUR package before installation".into(),
            arguments: vec![
                PromptArgument { name: "package_name".into(), description: "Name of the AUR package to audit".into(), required: true },
            ],
        },
        Prompt {
            name: "analyze_dependencies".into(),
            description: "Analyze package dependencies and suggest installation order".into(),
            arguments: vec![
                PromptArgument { name: "package_name".into(), description: "Name of the package to analyze dependencies for".into(), required: true },
            ],
        },
        Prompt {
            name: "safe_system_update".into(),
            description: "Enhanced system update workflow that checks for critical news, disk space, and failed services before updating".into(),
            arguments: vec![],
        },
        Prompt {
            name: "cleanup_system".into(),
            description: "Comprehensive system cleanup workflow: remove orphans, clean cache, verify integrity".into(),
            arguments: vec![
                PromptArgument { name: "aggressive".into(), description: "Perform aggressive cleanup (removes more packages). Default: false".into(), required: false },
            ],
        },
        Prompt {
            name: "package_investigation".into(),
            description: "Deep package research before installation: check repos, analyze security, review dependencies".into(),
            arguments: vec![
                PromptArgument { name: "package_name".into(), description: "Package name to investigate".into(), required: true },
            ],
        },
        Prompt {
            name: "mirror_optimization".into(),
            description: "Test and configure fastest mirrors based on location and latency".into(),
            arguments: vec![
                PromptArgument { name: "country".into(), description: "Country code for mirror suggestions (e.g., US, DE, JP)".into(), required: false },
            ],
        },
        Prompt {
            name: "system_health_check".into(),
            description: "Comprehensive system diagnostic: check disk, services, logs, database, integrity".into(),
            arguments: vec![],
        },
    ]
}

pub async fn get(name: &str, args: HashMap<String, Value>) -> Result<Vec<PromptMessage>, ToolError> {
    match name {
        "troubleshoot_issue" => get_troubleshoot(args).await,
        "audit_aur_package" => get_audit_aur(args).await,
        "analyze_dependencies" => get_analyze_deps(args).await,
        "safe_system_update" => get_safe_update().await,
        "cleanup_system" => get_cleanup(args),
        "package_investigation" => get_package_investigation(args),
        "mirror_optimization" => get_mirror_optimization(args),
        "system_health_check" => get_system_health(),
        _ => Err(ToolError::invalid_argument(&format!("Unknown prompt: {name}"))),
    }
}

async fn get_troubleshoot(args: HashMap<String, Value>) -> Result<Vec<PromptMessage>, ToolError> {
    let error_msg = args.get("error_message").and_then(|v| v.as_str()).unwrap_or("Unknown error");
    let context = args.get("context").and_then(|v| v.as_str()).unwrap_or("");

    let user_msg = PromptMessage {
        role: "user".into(),
        content: PromptContent { content_type: "text".into(), text: format!("I'm experiencing this error: {error_msg}\n\nContext: {context}\n\nPlease help me troubleshoot this issue using Arch Linux knowledge.") },
    };

    let mut msgs = vec![user_msg];
    msgs.push(PromptMessage {
        role: "assistant".into(),
        content: PromptContent { content_type: "text".into(), text: "I'll help diagnose this issue. Let me search the Arch Wiki for relevant information.".into() },
    });

    Ok(msgs)
}

async fn get_audit_aur(args: HashMap<String, Value>) -> Result<Vec<PromptMessage>, ToolError> {
    let pkg = args.get("package_name").and_then(|v| v.as_str()).unwrap_or("unknown");

    let result = if !pkg.is_empty() && pkg != "unknown" {
        // Fetch AUR info and PKGBUILD
        let info_url = format!("https://aur.archlinux.org/rpc/v5/info/{}", super::resources::urlencode(pkg));
        let info: serde_json::Value = crate::client::get_json(&info_url).await.unwrap_or(serde_json::json!({"error": "fetch failed"}));

        let pkgbuild_url = format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}", super::resources::urlencode(pkg));
        let pkgbuild = crate::client::get_text(&pkgbuild_url).await.unwrap_or_default();

        let metadata_risk = crate::tools::aur::AuditPackageSecurity.call(HashMap::from([
            ("action".into(), Value::String("metadata_risk".into())),
            ("package_name".into(), Value::String(pkg.into())),
            ("package_info".into(), info),
        ])).await.unwrap_or(serde_json::json!({"trust_score": 0}));

        let pkgbuild_analysis = crate::tools::aur::AuditPackageSecurity.call(HashMap::from([
            ("action".into(), Value::String("pkgbuild_analysis".into())),
            ("pkgbuild_content".into(), Value::String(pkgbuild)),
        ])).await.unwrap_or(serde_json::json!({"risk_score": 0}));

        format!(
            "# Security Audit Report for {pkg}\n\n## Metadata Analysis\nTrust Score: {}/100\n\n## PKGBUILD Analysis\nRisk Score: {}/100",
            metadata_risk.get("trust_score").and_then(|v| v.as_i64()).unwrap_or(0),
            pkgbuild_analysis.get("risk_score").and_then(|v| v.as_i64()).unwrap_or(0),
        )
    } else {
        format!("Package '{pkg}' not found or error occurred.")
    };

    Ok(vec![
        PromptMessage { role: "user".into(), content: PromptContent { content_type: "text".into(), text: format!("Please audit the AUR package '{pkg}' for security issues before installation.") } },
        PromptMessage { role: "assistant".into(), content: PromptContent { content_type: "text".into(), text: result } },
    ])
}

async fn get_analyze_deps(args: HashMap<String, Value>) -> Result<Vec<PromptMessage>, ToolError> {
    let pkg = args.get("package_name").and_then(|v| v.as_str()).unwrap_or("unknown");
    let mut analysis = format!("# Dependency Analysis for {pkg}\n\n");

    if !pkg.is_empty() && pkg != "unknown" {
        let mut pkg_args = HashMap::new();
        pkg_args.insert("package_name".into(), Value::String(pkg.into()));
        match crate::tools::pacman::GetOfficialPackageInfo.call(pkg_args).await {
            Ok(info) => {
                let deps: Vec<String> = info.get("dependencies").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
                let opt_deps: Vec<String> = info.get("optional_dependencies").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default();
                analysis.push_str("## Required Dependencies\n");
                for d in &deps { analysis.push_str(&format!("- {d}\n")); }
                if deps.is_empty() { analysis.push_str("None\n"); }
                analysis.push_str("\n## Optional Dependencies\n");
                for d in &opt_deps { analysis.push_str(&format!("- {d}\n")); }
                if opt_deps.is_empty() { analysis.push_str("None\n"); }
                analysis.push_str("\n## Installation\n```bash\nsudo pacman -S {pkg}\n```\n");
            }
            Err(_) => {
                analysis.push_str("Package not found in official repositories. Check AUR instead.\n");
                analysis.push_str("\n⚠️ **Important**: Always audit AUR packages for security before installation!\n");
            }
        }
    }

    Ok(vec![
        PromptMessage { role: "user".into(), content: PromptContent { content_type: "text".into(), text: format!("Please analyze the dependencies for the package '{pkg}' and suggest the best installation approach.") } },
        PromptMessage { role: "assistant".into(), content: PromptContent { content_type: "text".into(), text: analysis } },
    ])
}

async fn get_safe_update() -> Result<Vec<PromptMessage>, ToolError> {
    if !is_arch_linux() {
        return Ok(vec![PromptMessage { role: "assistant".into(), content: PromptContent { content_type: "text".into(), text: "Safe system update workflow is only available on Arch Linux.".into() } }]);
    }

    let mut analysis = String::from("# Safe System Update Workflow\n\n");
    let mut warnings: Vec<String> = Vec::new();

    // Check for critical news
    match crate::tools::news::FetchNews.call(HashMap::from([("action".into(), Value::String("critical".into()))])).await {
        Ok(news) => {
            let count = news.get("count").and_then(|v| v.as_i64()).unwrap_or(0);
            if count > 0 {
                analysis.push_str("## ⚠️ Critical Arch Linux News\n\nSome critical news items require attention before updating.\n\n");
                warnings.push("Critical news requiring manual intervention found!".into());
            } else {
                analysis.push_str("## ✓ No Critical News\n\nNo manual intervention required.\n\n");
            }
        }
        Err(_) => analysis.push_str("## ⚠️ News Check Failed\n\nCould not fetch news.\n\n"),
    }

    // Check disk space
    match crate::command::run("df", &["-h", "/"]).await {
        Ok(r) => analysis.push_str(&format!("## Disk Space\n\n```\n{}\n```\n\n", r.stdout)),
        Err(_) => analysis.push_str("## ⚠️ Disk Check Failed\n\n"),
    }

    // Check failed services
    match crate::command::run("systemctl", &["--failed"]).await {
        Ok(r) => {
            let failed = r.stdout.lines().count().saturating_sub(1);
            if failed > 0 {
                analysis.push_str(&format!("## ⚠️ {failed} Failed Services\n\n```\n{}\n```\n\n", r.stdout));
                warnings.push("System has failed services".into());
            } else {
                analysis.push_str("## ✓ All Services Running\n\n");
            }
        }
        Err(_) => {}
    }

    // Check pending updates
    match crate::command::run("checkupdates", &[]).await {
        Ok(r) => {
            let count = r.stdout.lines().count();
            if count > 0 {
                analysis.push_str(&format!("## Pending Updates ({count} packages)\n\n```\n{}\n```\n\n", r.stdout));
            } else {
                analysis.push_str("## ✓ System Up to Date\n\n");
            }
        }
        Err(_) => analysis.push_str("## ℹ️ Could not check pending updates.\n\n"),
    }

    if warnings.is_empty() {
        analysis.push_str("✓ System is ready for update\n\nRun: `sudo pacman -Syu`\n");
    } else {
        analysis.push_str("⚠️ **Address warnings before updating**\n");
        for w in &warnings {
            analysis.push_str(&format!("- ⚠️ {w}\n"));
        }
    }

    Ok(vec![
        PromptMessage { role: "user".into(), content: PromptContent { content_type: "text".into(), text: "Check if my system is ready for a safe update".into() } },
        PromptMessage { role: "assistant".into(), content: PromptContent { content_type: "text".into(), text: analysis } },
    ])
}

fn get_cleanup(args: HashMap<String, Value>) -> Result<Vec<PromptMessage>, ToolError> {
    let aggressive = args.get("aggressive").and_then(|v| v.as_str()).unwrap_or("false");
    let is_agg = aggressive == "true" || aggressive == "yes";

    let text = format!(
        "Please perform a comprehensive system cleanup:\n\n\
1. **Check Orphaned Packages**:\n   - Run manage_orphans with action='list'\n   - Review the list for packages that can be safely removed\n\
   {}\n\n\
2. **Clean Package Cache**:\n   - Run analyze_storage with action='cache_stats'\n   - If cache is large, suggest cleanup\n\n\
3. **Verify Package Integrity**:\n   - Run manage_orphans with action='list'\n   - Report issues\n\n\
4. **Check Database Freshness**:\n   - Run check_database_freshness\n   - If stale, suggest sync\n\n\
5. **Summary**:\n   - Space freed (estimate)\n   - Packages removed\n   - Recommended next steps",
        if is_agg { "   - Be aggressive: remove all orphans unless critical" } else { "   - Be conservative: keep packages that might be useful" }
    );

    Ok(vec![
        PromptMessage { role: "user".into(), content: PromptContent { content_type: "text".into(), text } },
    ])
}

fn get_package_investigation(args: HashMap<String, Value>) -> Result<Vec<PromptMessage>, ToolError> {
    let pkg = args.get("package_name").and_then(|v| v.as_str()).unwrap_or("");

    if pkg.is_empty() {
        return Ok(vec![
            PromptMessage { role: "assistant".into(), content: PromptContent { content_type: "text".into(), text: "Error: package_name argument is required".into() } },
        ]);
    }

    let text = format!(
        "Please investigate the package '{pkg}' thoroughly before installation:\n\n\
1. **Check Official Repositories First**:\n   - Run get_official_package_info(\"{pkg}\")\n   - If found in official repos: ✅ SAFE - recommend using pacman\n   - If not found: Continue to AUR investigation\n\n\
2. **Search AUR** (if not in official repos):\n   - Run search_aur(\"{pkg}\")\n   - Review: votes, popularity, maintainer, last update\n\n\
3. **Security Analysis**:\n   - For top AUR result, run audit_package_security with action='metadata_risk'\n   - Trust score interpretation:\n     - 80-100: Highly trusted\n     - 60-79: Generally safe\n     - 40-59: Review carefully\n     - 0-39: High risk, manual audit required\n\n\
4. **PKGBUILD Audit** (if proceeding with AUR):\n   - Fetch PKGBUILD content\n   - Run audit_package_security with action='pkgbuild_analysis'\n\n\
5. **Check Dependencies**:\n   - Review dependencies\n   - Warn about deep AUR dependency chains\n\n\
6. **Final Recommendation**:\n   - ✅ Safe to install (with command)\n   - ⚠️ Proceed with caution (explain risks)\n   - ⛔ Do not install (explain why)"
    );

    Ok(vec![
        PromptMessage { role: "user".into(), content: PromptContent { content_type: "text".into(), text } },
    ])
}

fn get_mirror_optimization(args: HashMap<String, Value>) -> Result<Vec<PromptMessage>, ToolError> {
    let country = args.get("country").and_then(|v| v.as_str()).unwrap_or("");
    let country_hint = if country.is_empty() { String::new() } else { format!(", country=\"{country}\"") };

    let text = format!(
        "Please optimize repository mirrors:\n\n\
1. **List and Test Current Mirrors**:\n   - Run optimize_mirrors(action='status', auto_test=True)\n   - Show currently configured mirrors with their speeds\n   - Identify slow mirrors (> 500ms)\n\n\
2. **Suggest Optimal Mirrors**:\n   - Run optimize_mirrors(action='suggest'{country_hint}, limit=10)\n   - Based on geographic location and current status\n   - Show top 10 recommended mirrors\n\n\
3. **Health Check**:\n   - Run optimize_mirrors(action='health')\n   - Identify any configuration issues\n\n\
4. **Recommendations**:\n   - Suggest mirror configuration changes\n   - Provide commands to update /etc/pacman.d/mirrorlist\n   - Recommend using reflector or manual configuration\n\n\
5. **Expected Benefits**:\n   - Estimate download speed improvements\n   - Reduced update times\n   - Better reliability"
    );

    Ok(vec![
        PromptMessage { role: "user".into(), content: PromptContent { content_type: "text".into(), text } },
    ])
}

fn get_system_health() -> Result<Vec<PromptMessage>, ToolError> {
    let text = "Please perform a comprehensive system health diagnostic:\n\n\
1. **System Information**:\n   - Run get_system_info\n   - Review kernel version, uptime, memory usage\n\n\
2. **Disk Space Analysis**:\n   - Run analyze_storage with action='disk_usage'\n   - Run analyze_storage with action='cache_stats'\n\n\
3. **Service Health**:\n   - Run diagnose_system with action='failed_services'\n   - If failures found, run diagnose_system with action='boot_logs'\n\n\
4. **Package Database Health**:\n   - Run check_database_freshness\n   - Identify any package operation failures\n\n\
5. **Package Integrity**:\n   - Run manage_orphans with action='list'\n   - Count orphaned packages\n\n\
6. **Configuration Health**:\n   - Run analyze_pacman_conf with focus='full'\n\n\
7. **Mirror Health**:\n   - Run optimize_mirrors with action='health'\n\n\
8. **Summary Report**:\n   - Overall health status (Healthy/Warnings/Critical)\n   - List of issues found with severity levels\n   - Prioritized recommendations for fixes";

    Ok(vec![
        PromptMessage { role: "user".into(), content: PromptContent { content_type: "text".into(), text: text.into() } },
    ])
}
