use std::path::Path;
use std::sync::OnceLock;

pub fn is_arch_linux() -> bool {
    static IS_ARCH: OnceLock<bool> = OnceLock::new();
    *IS_ARCH.get_or_init(|| {
        Path::new("/etc/arch-release").exists()
            || std::fs::read_to_string("/etc/os-release")
                .ok()
                .map(|content| content.contains("ID=arch") || content.contains("ID_LIKE=arch"))
                .unwrap_or(false)
    })
}

pub fn cpu_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else {
        "unknown"
    }
}
