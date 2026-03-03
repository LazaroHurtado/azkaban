use color_eyre::eyre::{Result, WrapErr};
use std::process::Command;

use crate::config::{CliToolConfig, ProjectConfig};
use crate::session;
use crate::worktree::WorktreeInfo;

/// Launch a CLI tool inside the Docker container, handing off the terminal.
pub fn launch_tool(
    container_name: &str,
    project: &ProjectConfig,
    worktree: &WorktreeInfo,
    tool_config: &CliToolConfig,
) -> Result<()> {
    // Determine the working directory inside the container
    let container_workdir = if worktree.is_main {
        format!("/workspace/{}", project.name)
    } else {
        let relative = worktree
            .path
            .strip_prefix(&project.path)
            .unwrap_or(&worktree.path);
        format!("/workspace/{}/{}", project.name, relative.display())
    };

    // Build the CLI tool command
    let cmd_parts = session::build_tool_command(&container_workdir, tool_config);

    // Run docker exec with inherited stdio for full interactive terminal
    let status = Command::new("docker")
        .args(["exec", "-it", container_name])
        .args(&cmd_parts)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .wrap_err("Failed to exec into container")?;

    if !status.success() {
        eprintln!(
            "CLI tool exited with status: {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

/// Resume a specific session inside the Docker container.
pub fn resume_session(
    container_name: &str,
    project: &ProjectConfig,
    worktree: &WorktreeInfo,
    tool_config: &CliToolConfig,
    session_id: &str,
) -> Result<()> {
    let container_workdir = if worktree.is_main {
        format!("/workspace/{}", project.name)
    } else {
        let relative = worktree
            .path
            .strip_prefix(&project.path)
            .unwrap_or(&worktree.path);
        format!("/workspace/{}/{}", project.name, relative.display())
    };

    let cmd_parts = session::build_resume_command(&container_workdir, tool_config, session_id);

    let status = Command::new("docker")
        .args(["exec", "-it", container_name])
        .args(&cmd_parts)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .wrap_err("Failed to exec into container")?;

    if !status.success() {
        eprintln!(
            "CLI tool exited with status: {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}
