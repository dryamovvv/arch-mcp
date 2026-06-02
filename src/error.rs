use std::fmt;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ErrorKind {
    PlatformNotArch,
    PacmanFailed(String),
    NetworkError(String),
    ParseError(String),
    InvalidArgument(String),
    Timeout,
    PermissionDenied,
    Internal(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolError {
    pub kind: ErrorKind,
    pub message: String,
    pub wiki_suggestions: Vec<String>,
}

impl ToolError {
    pub fn platform_not_arch() -> Self {
        ToolError {
            kind: ErrorKind::PlatformNotArch,
            message: "This tool requires Arch Linux (or Arch Linux ARM). The host system does not appear to be running Arch Linux.".into(),
            wiki_suggestions: vec!["Installation guide".into(), "Arch Linux on ARM".into()],
        }
    }

    pub fn pacman_failed(cmd: &str, detail: &str) -> Self {
        ToolError {
            kind: ErrorKind::PacmanFailed(cmd.into()),
            message: format!("`{}` failed: {}", cmd, detail),
            wiki_suggestions: vec!["Pacman troubleshooting".into()],
        }
    }

    pub fn invalid_argument(detail: &str) -> Self {
        ToolError {
            kind: ErrorKind::InvalidArgument(detail.into()),
            message: detail.into(),
            wiki_suggestions: vec![],
        }
    }

    pub fn network_error(detail: &str) -> Self {
        ToolError {
            kind: ErrorKind::NetworkError(detail.into()),
            message: format!("Network request failed: {}", detail),
            wiki_suggestions: vec!["Network configuration".into()],
        }
    }

    pub fn parse_error(detail: &str) -> Self {
        ToolError {
            kind: ErrorKind::ParseError(detail.into()),
            message: format!("Parse error: {}", detail),
            wiki_suggestions: vec![],
        }
    }

    pub fn internal(detail: &str) -> Self {
        ToolError {
            kind: ErrorKind::Internal(detail.into()),
            message: format!("Internal error: {}", detail),
            wiki_suggestions: vec![],
        }
    }
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.kind, self.message)
    }
}

impl std::error::Error for ToolError {}
