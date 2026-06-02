use crate::error::ToolError;
use crate::protocol::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub mod wiki;
pub mod aur;
pub mod pacman;
pub mod packages;
pub mod files;
pub mod system;
pub mod health;
pub mod news;
pub mod logs;
pub mod journal;
pub mod mirrors;
pub mod config;
pub mod btrfs;
pub mod boot;
pub mod report;
pub mod build_test;
pub mod luks;
pub mod firewall;
pub mod hardware;
pub mod boot_config;
pub mod backup;
pub mod recovery;
pub mod telegram;

#[async_trait]
pub trait ToolHandler: Send + Sync {
    fn info(&self) -> Tool;
    async fn call(&self, args: HashMap<String, Value>) -> Result<Value, ToolError>;
}

pub struct ToolEntry {
    pub info: Tool,
    pub handler: Box<dyn ToolHandler>,
}

pub struct ToolRegistry {
    tools: Vec<ToolEntry>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut reg = ToolRegistry { tools: Vec::new() };

        macro_rules! register {
            ($handler:expr) => {
                let h = $handler;
                let info = h.info();
                reg.tools
                    .push(ToolEntry { info, handler: Box::new(h) });
            };
        }

        register!(wiki::SearchArchWiki);
        register!(aur::SearchAur);
        register!(aur::AuditPackageSecurity);
        register!(aur::InstallPackageSecure);
        register!(pacman::GetOfficialPackageInfo);
        register!(pacman::CheckUpdatesDryRun);
        register!(pacman::RemovePackages);
        register!(packages::ManageOrphans);
        register!(packages::ManageInstallReason);
        register!(packages::VerifyPackageIntegrity);
        register!(packages::CheckDatabaseFreshness);
        register!(files::QueryFileOwnership);
        register!(files::ManageGroups);
        register!(system::GetSystemInfo);
        register!(system::AnalyzeStorage);
        register!(system::DiagnoseSystem);
        register!(health::RunSystemHealthCheck);
        register!(news::FetchNews);
        register!(logs::QueryPackageHistory);
        register!(journal::ManageLogs);
        register!(journal::ManageJournalGateway);
        register!(mirrors::OptimizeMirrors);
        register!(config::AnalyzePacmanConf);
        register!(config::AnalyzeMakepkgConf);
        register!(btrfs::AnalyzeBtrfs);
        register!(btrfs::ManageBtrfsSnapshots);
        register!(btrfs::ManageBtrfsScrub);
        register!(boot::ManageBoot);
        register!(report::GenerateReport);
        register!(build_test::VerifyBootArtifacts);
        register!(build_test::VerifyServiceHealth);
        register!(build_test::VerifyHomectlUser);
        register!(build_test::CompareFstab);
        register!(build_test::ComparePackages);
        register!(build_test::CheckSecurityPosture);
        register!(build_test::CheckRpiHardware);
        register!(build_test::BenchmarkQuick);
        register!(luks::ManageLuks);
        register!(firewall::ManageFirewall);
        register!(hardware::ManageHardware);
        register!(boot_config::ManageBootConfig);
        register!(backup::ManageBackup);
        register!(recovery::ManageRecovery);
        register!(telegram::ManageTelegramUnlock);

        reg
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools.iter().map(|e| e.info.clone()).collect()
    }

    pub async fn call(
        &self,
        name: &str,
        args: HashMap<String, Value>,
    ) -> Result<Value, ToolError> {
        let entry = self
            .tools
            .iter()
            .find(|e| e.info.name == name)
            .ok_or_else(|| ToolError::invalid_argument(&format!("Unknown tool: {}", name)))?;
        entry.handler.call(args).await
    }
}
