crate::stub_tool!(VerifyBootArtifacts, "verify_boot_artifacts", "Verify RPi5 boot artifacts (kernel, DTB, config)");
crate::stub_tool!(VerifyServiceHealth, "verify_service_health", "Verify systemd service health against a checklist");
crate::stub_tool!(VerifyHomectlUser, "verify_homectl_user", "Verify systemd-homed user configuration");
crate::stub_tool!(CompareFstab, "compare_fstab", "Compare fstab against expected RPi5 BTRFS layout");
crate::stub_tool!(ComparePackages, "compare_packages", "Compare installed packages against build configuration");
crate::stub_tool!(CheckSecurityPosture, "check_security_posture", "Check SSH, fail2ban, sudo, MCP security posture");
crate::stub_tool!(CheckRpiHardware, "check_rpi_hardware", "Check RPi5 hardware via vcgencmd");
crate::stub_tool!(BenchmarkQuick, "benchmark_quick", "Run quick benchmarks (disk, CPU, memory)");
