use crate::error::ToolError;
use crate::platform::is_arch_linux;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

use super::ToolHandler;

const AUR_RPC_URL: &str = "https://aur.archlinux.org/rpc/v5";
const AUR_CGIT_URL: &str = "https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD";

// --- Search AUR ---

pub struct SearchAur;

#[async_trait]
impl ToolHandler for SearchAur {
    fn info(&self) -> Tool {
        Tool {
            name: "search_aur".into(),
            description: "Search the Arch User Repository (AUR) for packages matching the given query".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search term" },
                    "limit": { "type": "integer", "default": 10 },
                    "sort_by": {
                        "type": "string",
                        "enum": ["relevance", "votes", "popularity", "updated"],
                        "default": "relevance"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: query"))?;
        let limit = args
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;
        let sort_by = args
            .get("sort_by")
            .and_then(|v| v.as_str())
            .unwrap_or("relevance");

        search_aur(query, limit, sort_by).await
    }
}

#[derive(Deserialize)]
struct AurResponse {
    #[serde(rename = "type")]
    resp_type: String,
    results: Vec<AurPackage>,
    error: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
struct AurPackage {
    name: Option<String>,
    package_base: Option<String>,
    version: Option<String>,
    description: Option<String>,
    num_votes: Option<i32>,
    popularity: Option<f64>,
    maintainer: Option<String>,
    out_of_date: Option<i64>,
    first_submitted: Option<i64>,
    last_modified: Option<i64>,
    url: Option<String>,
    license: Option<String>,
    keywords: Option<Vec<String>>,
}

async fn search_aur(query: &str, limit: usize, sort_by: &str) -> Result<Value, ToolError> {
    let url = format!("{}/search/{}?by=name-desc", AUR_RPC_URL, urlencode(query));
    let resp: AurResponse = crate::client::get_json(&url).await?;

    if resp.resp_type == "error" {
        return Err(ToolError::network_error(
            &resp.error.unwrap_or("AUR error".into()),
        ));
    }

    let mut packages = resp.results;
    sort_packages(&mut packages, sort_by);
    packages.truncate(limit);

    let results: Vec<Value> = packages
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name.unwrap_or_default(),
                "package_base": p.package_base,
                "version": p.version,
                "description": p.description,
                "votes": p.num_votes,
                "popularity": p.popularity,
                "maintainer": p.maintainer,
                "out_of_date": p.out_of_date,
                "first_submitted": p.first_submitted,
                "last_modified": p.last_modified,
                "url": p.url,
                "license": p.license,
                "keywords": p.keywords,
            })
        })
        .collect();

    Ok(serde_json::json!({ "results": results, "count": results.len(), "query": query }))
}

fn sort_packages(pkgs: &mut [AurPackage], sort_by: &str) {
    match sort_by {
        "votes" => pkgs.sort_by(|a, b| b.num_votes.unwrap_or(0).cmp(&a.num_votes.unwrap_or(0))),
        "popularity" => pkgs.sort_by(|a, b| {
            b.popularity
                .partial_cmp(&a.popularity)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
        "updated" => pkgs.sort_by(|a, b| {
            b.last_modified
                .unwrap_or(0)
                .cmp(&a.last_modified.unwrap_or(0))
        }),
        _ => {}
    }
}

fn urlencode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            ' ' => "+".into(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

// --- Security Audit ---

pub struct AuditPackageSecurity;

#[async_trait]
impl ToolHandler for AuditPackageSecurity {
    fn info(&self) -> Tool {
        Tool {
            name: "audit_package_security".into(),
            description: "Audit AUR package security by analyzing PKGBUILD content or metadata risk".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["pkgbuild_analysis", "metadata_risk"],
                        "description": "Analysis type"
                    },
                    "pkgbuild_content": {
                        "type": "string",
                        "description": "PKGBUILD content for pkgbuild_analysis"
                    },
                    "package_name": { "type": "string" },
                    "package_info": {
                        "type": "object",
                        "description": "Package metadata for risk scoring"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: action"))?;

        match action {
            "pkgbuild_analysis" => {
                let content = args
                    .get("pkgbuild_content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::invalid_argument("Missing: pkgbuild_content"))?;
                Ok(analyze_pkgbuild(content))
            }
            "metadata_risk" => {
                let name = args
                    .get("package_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let info = args
                    .get("package_info")
                    .cloned()
                    .unwrap_or(Value::Null);
                Ok(metadata_risk_score(name, &info))
            }
            _ => Err(ToolError::invalid_argument("Unknown action")),
        }
    }
}

fn analyze_pkgbuild(content: &str) -> Value {
    let mut red_flags: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    let red_patterns = [
        (r"rm\s+-rf\s+(/|/\*)", "Dangerous rm -rf on root"),
        (
            r":\{?\s*\)\s*\{",
            "Possible fork bomb",
        ),
        (r"base64\s+-d", "Base64 decode - possible obfuscation"),
        (r"curl\s+.*\|\s*(bash|sh)", "curl pipe to shell"),
        (r"wget\s+.*\|\s*(bash|sh)", "wget pipe to shell"),
        (
            r"(LD_PRELOAD|LD_LIBRARY_PATH)\s*=",
            "LD_PRELOAD/LD_LIBRARY_PATH manipulation",
        ),
        (
            r"(chmod|chown)\s+-R\s+777\s+/",
            "Recursive 777 on root",
        ),
        (
            r"(Bitcoin|Monero|cryptonight|ethminer|xmrig)",
            "Cryptocurrency mining reference",
        ),
        (
            r"(reverse[_ ]?shell|backconnect)",
            "Reverse shell reference",
        ),
        (r"eval\s+\$?\(", "Eval with command substitution"),
        (r"exec\s+[^s]", "Unchecked exec"),
        (r"source\s+\$\(curl", "Source from curl"),
        (r"\.\s+\$\(wget", "Source from wget"),
        (r"/dev/tcp/", "Bash TCP connection"),
        (r"chown\s+.*\s+/usr", "Chown on /usr"),
        (r"chmod\s+777\s+", "World-writable permissions"),
        (
            r"pkexec|sudo\s+",
            "Privilege escalation in PKGBUILD",
        ),
    ];

    for (pattern, desc) in &red_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(content) {
                red_flags.push(desc.to_string());
            }
        }
    }

    let warn_patterns = [
        (
            r"https?://[^\s]*(?:bit\.ly|tinyurl|goo\.gl)",
            "URL shortener",
        ),
        (
            r"(\.tar\.gz|\.zip|\.7z)\s*$",
            "Binary archive (no source)",
        ),
        (
            r"(noextract|skipinteg)",
            "Skipping integrity checks",
        ),
    ];

    for (pattern, desc) in &warn_patterns {
        if let Ok(re) = regex::Regex::new(pattern) {
            if re.is_match(content) {
                warnings.push(desc.to_string());
            }
        }
    }

    let red_score = (red_flags.len() * 50).min(100) as u32;
    let warn_score = (warnings.len() * 5).min((100 - red_score) as usize) as u32;
    let risk_score = red_score + warn_score;

    let risk_level = if risk_score >= 70 {
        "HIGH"
    } else if risk_score >= 30 {
        "MEDIUM"
    } else {
        "LOW"
    };

    serde_json::json!({
        "risk_score": risk_score,
        "risk_level": risk_level,
        "red_flags": red_flags,
        "warnings": warnings,
        "summary": format!("{} red flags, {} warnings found", red_flags.len(), warnings.len())
    })
}

fn metadata_risk_score(_name: &str, info: &Value) -> Value {
    let votes = info
        .get("num_votes")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let popularity = info
        .get("popularity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let has_maintainer = info
        .get("maintainer")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    let trust_score = {
        let mut score = 0i64;
        if votes >= 100 {
            score += 30;
        } else if votes >= 10 {
            score += 20;
        } else if votes > 0 {
            score += 10;
        }

        if popularity >= 10.0 {
            score += 25;
        } else if popularity >= 1.0 {
            score += 15;
        } else if popularity > 0.0 {
            score += 5;
        }

        if has_maintainer {
            score += 25;
        }

        score.min(100)
    };

    let risk_level = if trust_score >= 70 {
        "LOW"
    } else if trust_score >= 40 {
        "MEDIUM"
    } else {
        "HIGH"
    };

    serde_json::json!({
        "trust_score": trust_score,
        "risk_level": risk_level,
        "factors": {
            "votes": votes,
            "popularity": popularity,
            "has_maintainer": has_maintainer
        }
    })
}

// --- Install Package Secure ---

pub struct InstallPackageSecure;

#[async_trait]
impl ToolHandler for InstallPackageSecure {
    fn info(&self) -> Tool {
        Tool {
            name: "install_package_secure".into(),
            description: "Securely install an AUR package with pre-installation analysis".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "package_name": {
                        "type": "string",
                        "description": "Package to install"
                    }
                },
                "required": ["package_name"]
            }),
        }
    }

    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError> {
        if !is_arch_linux() {
            return Err(ToolError::platform_not_arch());
        }

        let pkg = args
            .get("package_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_argument("Missing: package_name"))?;

        let official_url = format!(
            "https://archlinux.org/packages/search/json/?name={}",
            urlencode(pkg)
        );
        let official: Value = crate::client::get_json(&official_url).await?;
        if official
            .get("results")
            .and_then(|r| r.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false)
        {
            return Ok(serde_json::json!({
                "status": "skipped",
                "message": format!("{} is in official repos - use pacman instead of AUR", pkg),
                "official": true
            }));
        }

        let aur_url = format!("{}/info/{}", AUR_RPC_URL, urlencode(pkg));
        let aur_info: AurResponse = crate::client::get_json(&aur_url).await?;
        if aur_info.resp_type == "error" || aur_info.results.is_empty() {
            return Ok(serde_json::json!({
                "status": "not_found",
                "message": format!("{} not found in AUR", pkg)
            }));
        }

        let info = &aur_info.results[0];
        let pkgbuild_url = format!(
            "{}?h={}",
            AUR_CGIT_URL,
            urlencode(info.package_base.as_deref().unwrap_or(pkg))
        );
        let pkgbuild_content = crate::client::get_text(&pkgbuild_url).await?;
        let security = analyze_pkgbuild(&pkgbuild_content);
        let risk_level = security
            .get("risk_level")
            .and_then(|v| v.as_str())
            .unwrap_or("HIGH");

        if risk_level == "HIGH" {
            return Ok(serde_json::json!({
                "status": "blocked",
                "message": "Security analysis blocked installation due to HIGH risk level",
                "security_analysis": security,
                "package_name": pkg
            }));
        }

        let helper = detect_aur_helper();
        if let Some(h) = helper {
            let cmd_result = crate::command::run(&h, &["-S", pkg]).await?;
            Ok(serde_json::json!({
                "status": if cmd_result.exit_code == 0 { "installed" } else { "failed" },
                "helper": h,
                "exit_code": cmd_result.exit_code,
                "stdout": cmd_result.stdout,
                "stderr": cmd_result.stderr,
                "security_analysis": security,
                "package_name": pkg
            }))
        } else {
            Ok(serde_json::json!({
                "status": "no_helper",
                "message": "No AUR helper (paru/yay) found. Install one first.",
                "package_name": pkg
            }))
        }
    }
}

fn detect_aur_helper() -> Option<String> {
    for helper in &["paru", "yay"] {
        if std::process::Command::new("which")
            .arg(helper)
            .output()
            .is_ok()
        {
            return Some(helper.to_string());
        }
    }
    None
}
