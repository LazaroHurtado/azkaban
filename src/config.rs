use color_eyre::eyre::{Result, WrapErr};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub cli_tools: Vec<CliToolConfig>,
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,
    #[serde(default)]
    pub project_dirs: Vec<String>,
    /// Container name to exec into (must match docker-compose.yml)
    #[serde(default = "default_container_name")]
    pub container_name: String,
    /// Global default CLI tool name
    #[serde(default)]
    pub default_tool: Option<String>,

    /// Runtime-only: the resolved root directory (not serialized)
    #[serde(skip)]
    pub root_dir: PathBuf,
}

fn default_container_name() -> String {
    "azkaban-sandbox".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CliToolConfig {
    /// Short identifier used in default_tool references (e.g. "copilot")
    pub name: String,
    /// Display name shown in the TUI (e.g. "GitHub Copilot")
    #[serde(default)]
    pub display_name: Option<String>,
    /// npm/shell command to install this tool inside the Docker image
    #[serde(default)]
    pub install_cmd: Option<String>,
    /// The CLI binary to invoke (e.g. "copilot", "claude", "gemini")
    pub cli_cmd: String,
    /// Flags always appended to the CLI command (e.g. ["--yolo"])
    #[serde(default)]
    pub flags: Vec<String>,
}

impl CliToolConfig {
    pub fn display(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub default_tool: Option<String>,
}

impl Config {
    pub fn find_tool(&self, name: &str) -> Option<&CliToolConfig> {
        self.cli_tools.iter().find(|t| t.name == name)
    }

    /// Resolve the azkaban root directory from the executable's location.
    /// Walks up from the binary path until it finds a directory containing `config.yaml`.
    fn find_root() -> Result<PathBuf> {
        let exe = std::env::current_exe()
            .and_then(|p| p.canonicalize())
            .wrap_err("Failed to resolve executable path")?;

        let mut dir = exe.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            if d.join("config.yaml").exists() {
                return Ok(d);
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }

        color_eyre::eyre::bail!(
            "Could not find config.yaml in any parent of {}",
            exe.display()
        )
    }

    pub fn load() -> Result<Self> {
        let root = Self::find_root()?;
        let path = root.join("config.yaml");
        let content = std::fs::read_to_string(&path)
            .wrap_err_with(|| format!("Failed to read config from {}", path.display()))?;
        let mut config: Config =
            serde_yaml::from_str(&content).wrap_err("Failed to parse config.yaml")?;
        config.root_dir = root;
        config.expand_project_dirs();
        Ok(config)
    }

    /// Expand `project_dirs` glob patterns into individual projects.
    /// Each subdirectory matching the pattern becomes a project, using
    /// the directory name as the project name.
    fn expand_project_dirs(&mut self) {
        let existing_paths: std::collections::HashSet<String> = self
            .projects
            .iter()
            .map(|p| p.path.clone())
            .collect();

        for pattern in &self.project_dirs {
            // Expand ~ to home directory
            let expanded = if pattern.starts_with('~') {
                if let Some(home) = std::env::var_os("HOME") {
                    pattern.replacen('~', &home.to_string_lossy(), 1)
                } else {
                    pattern.clone()
                }
            } else {
                pattern.clone()
            };

            if let Ok(entries) = glob::glob(&expanded) {
                for entry in entries.flatten() {
                    if entry.is_dir() {
                        let path_str = entry.to_string_lossy().to_string();
                        if existing_paths.contains(&path_str) {
                            continue;
                        }
                        let name = entry
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        if !name.is_empty() && !name.starts_with('.') {
                            self.projects.push(ProjectConfig {
                                name,
                                path: path_str,
                                default_tool: None,
                            });
                        }
                    }
                }
            }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cli_tools: Vec::new(),
            projects: Vec::new(),
            project_dirs: Vec::new(),
            container_name: default_container_name(),
            default_tool: None,
            root_dir: PathBuf::new(),
        }
    }
}
