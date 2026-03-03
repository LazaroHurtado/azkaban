use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::Path;

use crate::config::CliToolConfig;

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub summary: String,
    #[allow(dead_code)]
    pub cwd: String,
    pub updated_at: DateTime<Utc>,
}

impl SessionInfo {
    /// Format the time since last update as a human-readable string.
    pub fn time_ago(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.updated_at);

        if duration.num_minutes() < 1 {
            "just now".to_string()
        } else if duration.num_minutes() < 60 {
            format!("{}m ago", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_days() < 30 {
            format!("{}d ago", duration.num_days())
        } else {
            format!("{}mo ago", duration.num_days() / 30)
        }
    }
}

/// Copilot workspace.yaml schema
#[derive(Debug, Deserialize)]
struct CopilotWorkspace {
    id: String,
    #[serde(default)]
    cwd: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    updated_at: Option<String>,
}

/// List all sessions from the Copilot session-state directory for a given worktree cwd.
/// `config_dir` is the host path to the copilot config (e.g., "configs/copilot").
/// `container_workdir` is the cwd path inside the container (e.g., "/workspace/myproject").
pub fn list_copilot_sessions(config_dir: &Path, container_workdir: &str) -> Vec<SessionInfo> {
    let session_state = config_dir.join("session-state");
    if !session_state.exists() {
        return Vec::new();
    }

    let mut sessions = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&session_state) {
        for entry in entries.flatten() {
            let workspace_file = entry.path().join("workspace.yaml");
            if !workspace_file.exists() {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(&workspace_file) {
                if let Ok(ws) = serde_yaml::from_str::<CopilotWorkspace>(&content) {
                    // Filter: only sessions for this worktree's cwd
                    if !container_workdir.is_empty() && ws.cwd != container_workdir {
                        continue;
                    }

                    let updated_at = ws
                        .updated_at
                        .as_deref()
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(Utc::now);

                    sessions.push(SessionInfo {
                        id: ws.id,
                        summary: if ws.summary.is_empty() {
                            "(no summary)".to_string()
                        } else {
                            ws.summary
                        },
                        cwd: ws.cwd,
                        updated_at,
                    });
                }
            }
        }
    }

    // Sort by most recent first
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    sessions
}

/// List all sessions across all tools for a given worktree.
/// Currently only supports Copilot.
pub fn list_sessions_for_worktree(
    root_dir: &Path,
    cli_tools: &[CliToolConfig],
    container_workdir: &str,
) -> Vec<SessionInfo> {
    let mut all_sessions = Vec::new();

    for tool in cli_tools {
        if tool.name == "copilot" {
            // Look for copilot sessions in configs/copilot/ relative to repo root
            let config_dir = root_dir.join("configs").join("copilot");
            all_sessions.extend(list_copilot_sessions(&config_dir, container_workdir));
        }
    }

    all_sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    all_sessions
}

/// Build the shell command to launch a CLI tool.
pub fn build_tool_command(
    workdir: &str,
    tool_config: &CliToolConfig,
) -> Vec<String> {
    let mut cmd = tool_config.cli_cmd.clone();

    for flag in &tool_config.flags {
        cmd.push(' ');
        cmd.push_str(flag);
    }

    vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("cd {} && {}", workdir, cmd),
    ]
}

/// Build the shell command to resume a specific session.
pub fn build_resume_command(
    workdir: &str,
    tool_config: &CliToolConfig,
    session_id: &str,
) -> Vec<String> {
    let mut cmd = tool_config.cli_cmd.clone();
    cmd.push_str(&format!(" --resume {}", session_id));

    for flag in &tool_config.flags {
        cmd.push(' ');
        cmd.push_str(flag);
    }

    vec![
        "sh".to_string(),
        "-c".to_string(),
        format!("cd {} && {}", workdir, cmd),
    ]
}
