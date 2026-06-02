#![allow(dead_code)]

use crate::error::ToolError;
use tokio::process::Command;

pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

pub async fn run(program: &str, args: &[&str]) -> Result<CommandOutput, ToolError> {
    let output = Command::new(program)
        .args(args)
        .output()
        .await
        .map_err(|e| ToolError::pacman_failed(program, &format!("exec: {}", e)))?;

    Ok(CommandOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

pub async fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout: std::time::Duration,
) -> Result<CommandOutput, ToolError> {
    let cmd = Command::new(program).args(args).output();
    tokio::time::timeout(timeout, cmd)
        .await
        .map_err(|_| ToolError::internal(&format!("`{}` timed out", program)))?
        .map_err(|e| ToolError::pacman_failed(program, &format!("exec: {}", e)))
        .map(|output| CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
}

pub async fn run_with_stdin(
    program: &str,
    args: &[&str],
    stdin_data: &str,
) -> Result<CommandOutput, ToolError> {
    use tokio::io::AsyncWriteExt;

    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.stdin(std::process::Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| ToolError::pacman_failed(program, &format!("spawn: {}", e)))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .await
            .map_err(|e| ToolError::pacman_failed(program, &format!("stdin: {}", e)))?;
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| ToolError::pacman_failed(program, &format!("wait: {}", e)))?;

    Ok(CommandOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}
