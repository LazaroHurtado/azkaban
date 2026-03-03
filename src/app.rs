use crate::config::{CliToolConfig, Config, ProjectConfig};
use crate::session::SessionInfo;
use crate::worktree::WorktreeInfo;

#[derive(Debug, Clone)]
pub enum Screen {
    ProjectList,
    WorktreeList {
        project_index: usize,
    },
    ToolSelect {
        project_index: usize,
        worktree: WorktreeInfo,
    },
    SessionList {
        project_index: usize,
        worktree_index: usize,
    },
    NewWorktree {
        project_index: usize,
        input: String,
    },
    ConfirmDelete {
        project_index: usize,
        worktree_index: usize,
    },
    NoGitWarning {
        project_index: usize,
        /// 0 = Return, 1 = Continue
        selected_button: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    LaunchTool {
        project: ProjectConfig,
        worktree: WorktreeInfo,
        cli: CliToolConfig,
    },
    ResumeSession {
        project: ProjectConfig,
        worktree: WorktreeInfo,
        cli: CliToolConfig,
        session_id: String,
    },
    Refresh,
}

pub struct App {
    pub config: Config,
    pub screen: Screen,
    pub list_index: usize,
    pub project_list_index: usize,
    pub worktree_list_index: usize,
    pub worktrees: Vec<WorktreeInfo>,
    pub sessions: Vec<SessionInfo>,
    pub status_message: Option<String>,
    pub search_query: String,
    pub filtered_indices: Vec<usize>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let filtered_indices: Vec<usize> = (0..config.projects.len()).collect();
        Self {
            config,
            screen: Screen::ProjectList,
            list_index: 0,
            project_list_index: 0,
            worktree_list_index: 0,
            worktrees: Vec::new(),
            sessions: Vec::new(),
            status_message: None,
            search_query: String::new(),
            filtered_indices,
        }
    }

    pub fn current_items_count(&self) -> usize {
        match &self.screen {
            Screen::ProjectList => self.filtered_indices.len(),
            // +1 for the "[+ New Worktree]" option
            Screen::WorktreeList { .. } => self.worktrees.len() + 1,
            Screen::ToolSelect { .. } => self.config.cli_tools.len(),
            Screen::SessionList { .. } => self.sessions.len() + 1, // +1 for "[+ New Session]"
            Screen::NewWorktree { .. } | Screen::ConfirmDelete { .. } | Screen::NoGitWarning { .. } => 0,
        }
    }

    pub fn move_up(&mut self) {
        if self.list_index > 0 {
            self.list_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let count = self.current_items_count();
        if count > 0 && self.list_index < count - 1 {
            self.list_index += 1;
        }
    }

    pub fn go_back(&mut self) {
        self.status_message = None;
        match &self.screen {
            Screen::ProjectList => {}
            Screen::WorktreeList { .. } => {
                self.screen = Screen::ProjectList;
                self.list_index = self.project_list_index;
                self.update_filter();
            }
            Screen::ToolSelect { project_index, .. } => {
                let pi = *project_index;
                self.screen = Screen::WorktreeList {
                    project_index: pi,
                };
                self.list_index = self.worktree_list_index;
            }
            Screen::SessionList { project_index, .. } => {
                let pi = *project_index;
                self.screen = Screen::WorktreeList {
                    project_index: pi,
                };
                self.list_index = self.worktree_list_index;
            }
            Screen::NewWorktree { project_index, .. }
            | Screen::ConfirmDelete { project_index, .. } => {
                let pi = *project_index;
                self.screen = Screen::WorktreeList {
                    project_index: pi,
                };
                self.list_index = self.worktree_list_index;
            }
            Screen::NoGitWarning { project_index, .. } => {
                let pi = *project_index;
                self.screen = Screen::ProjectList;
                self.list_index = pi;
                self.update_filter();
            }
        }
    }

    pub fn select(&mut self) -> Action {
        self.status_message = None;
        match &self.screen {
            Screen::ProjectList => {
                if self.filtered_indices.is_empty() {
                    self.status_message = Some(if self.search_query.is_empty() {
                        "No projects configured. Edit config.toml in the azkaban repo".to_string()
                    } else {
                        "No matching projects".to_string()
                    });
                    return Action::None;
                }
                let project_index = self.filtered_indices[self.list_index];
                self.project_list_index = self.list_index;
                let project = &self.config.projects[project_index];
                let project_path = std::path::Path::new(&project.path);

                match crate::worktree::list_worktrees(project_path) {
                    Ok(wts) => {
                        self.worktrees = wts;
                        self.screen = Screen::WorktreeList { project_index };
                        self.list_index = 0;
                    }
                    Err(_) => {
                        // No .git — show warning
                        if !project_path.join(".git").exists() {
                            self.screen = Screen::NoGitWarning { project_index, selected_button: 1 };
                        } else {
                            self.status_message = Some("Failed to list worktrees".to_string());
                        }
                    }
                }
                Action::None
            }
            Screen::WorktreeList { project_index } => {
                let pi = *project_index;
                if self.list_index == 0 {
                    // "[+ New Worktree]" selected
                    self.screen = Screen::NewWorktree {
                        project_index: pi,
                        input: String::new(),
                    };
                    self.list_index = 0;
                    Action::None
                } else {
                    // Existing worktree selected — show sessions panel
                    let worktree_index = self.list_index - 1;
                    self.worktree_list_index = self.list_index;
                    let worktree = &self.worktrees[worktree_index];
                    let project = &self.config.projects[pi];

                    // Compute the container workdir for this worktree
                    let container_workdir = if worktree.is_main {
                        format!("/workspace/{}", project.name)
                    } else {
                        let relative = worktree
                            .path
                            .strip_prefix(&project.path)
                            .unwrap_or(&worktree.path);
                        format!("/workspace/{}/{}", project.name, relative.display())
                    };

                    // Load sessions for this worktree
                    self.sessions = crate::session::list_sessions_for_worktree(
                        &self.config.root_dir,
                        &self.config.cli_tools,
                        &container_workdir,
                    );

                    self.screen = Screen::SessionList {
                        project_index: pi,
                        worktree_index,
                    };
                    self.list_index = 0;
                    Action::None
                }
            }
            Screen::ToolSelect {
                project_index,
                worktree,
            } => {
                if self.config.cli_tools.is_empty() {
                    self.status_message = Some("No CLI tools configured in config.toml".to_string());
                    return Action::None;
                }
                let tool_cfg = self.config.cli_tools[self.list_index].clone();
                let project = self.config.projects[*project_index].clone();
                let worktree = worktree.clone();
                Action::LaunchTool {
                    project,
                    worktree,
                    cli: tool_cfg,
                }
            }
            Screen::NewWorktree { .. } | Screen::ConfirmDelete { .. } | Screen::NoGitWarning { .. } | Screen::SessionList { .. } => Action::None,
        }
    }

    /// Select a session or start a new one from the SessionList screen.
    pub fn select_session(&mut self) -> Action {
        if let Screen::SessionList {
            project_index,
            worktree_index,
        } = self.screen
        {
            let project = &self.config.projects[project_index];
            let worktree = self.worktrees[worktree_index].clone();

            if self.list_index == 0 {
                // "[+ New Session]" — go to tool select or launch default
                if let Some(ref default_tool) = project.default_tool {
                    if let Some(tool_cfg) = self.config.find_tool(default_tool) {
                        return Action::LaunchTool {
                            project: project.clone(),
                            worktree,
                            cli: tool_cfg.clone(),
                        };
                    }
                }
                if let Some(ref default_tool) = self.config.default_tool {
                    if let Some(tool_cfg) = self.config.find_tool(default_tool) {
                        return Action::LaunchTool {
                            project: project.clone(),
                            worktree,
                            cli: tool_cfg.clone(),
                        };
                    }
                }
                // No default — show tool selector
                self.screen = Screen::ToolSelect {
                    project_index,
                    worktree,
                };
                self.list_index = 0;
                return Action::None;
            }

            // Resume existing session
            let session = &self.sessions[self.list_index - 1];
            let session_id = session.id.clone();

            // Find which tool to use — for now, default tool or copilot
            let tool_name = project
                .default_tool
                .as_deref()
                .or(self.config.default_tool.as_deref())
                .unwrap_or("copilot");

            if let Some(tool_cfg) = self.config.find_tool(tool_name) {
                return Action::ResumeSession {
                    project: project.clone(),
                    worktree,
                    cli: tool_cfg.clone(),
                    session_id,
                };
            }
        }
        Action::None
    }

    /// Handle a character input for the new worktree name input.
    pub fn handle_char(&mut self, c: char) -> Action {
        if let Screen::NewWorktree { ref mut input, .. } = self.screen {
            input.push(c);
        }
        Action::None
    }

    /// Handle backspace in input fields.
    pub fn handle_backspace(&mut self) {
        if let Screen::NewWorktree { ref mut input, .. } = self.screen {
            input.pop();
        }
    }

    /// Confirm the new worktree name and create it.
    pub fn confirm_new_worktree(&mut self) -> Action {
        if let Screen::NewWorktree {
            project_index,
            ref input,
        } = self.screen
        {
            let branch_name = input.trim().to_string();
            if branch_name.is_empty() {
                self.status_message = Some("Branch name cannot be empty".to_string());
                return Action::None;
            }

            let project = &self.config.projects[project_index];
            let project_path = std::path::Path::new(&project.path);

            match crate::worktree::create_worktree(project_path, &branch_name) {
                Ok(wt) => {
                    self.worktrees.push(wt);
                    self.screen = Screen::WorktreeList { project_index };
                    // Select the newly created worktree (last in list + 1 for the new option)
                    self.list_index = self.worktrees.len();
                    self.status_message = Some(format!("Created worktree: {branch_name}"));
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to create worktree: {e}"));
                }
            }
        }
        Action::None
    }

    /// Prompt to delete the selected worktree.
    pub fn delete_worktree(&mut self) -> Action {
        if let Screen::WorktreeList { project_index } = self.screen {
            if self.list_index == 0 {
                return Action::None;
            }
            let wt_index = self.list_index - 1;
            let worktree = &self.worktrees[wt_index];
            if worktree.is_main {
                self.status_message = Some("Cannot delete the main worktree".to_string());
                return Action::None;
            }

            self.screen = Screen::ConfirmDelete {
                project_index,
                worktree_index: wt_index,
            };
        }
        Action::None
    }

    /// Actually perform the deletion after confirmation.
    pub fn confirm_delete(&mut self) -> Action {
        if let Screen::ConfirmDelete {
            project_index,
            worktree_index,
        } = self.screen
        {
            let project = &self.config.projects[project_index];
            let project_path = std::path::Path::new(&project.path);
            let worktree_path = self.worktrees[worktree_index].path.clone();

            match crate::worktree::remove_worktree(project_path, &worktree_path) {
                Ok(()) => {
                    self.worktrees.remove(worktree_index);
                    self.screen = Screen::WorktreeList { project_index };
                    self.list_index = if self.worktrees.is_empty() { 0 } else { 1 };
                    self.status_message = Some("Worktree deleted".to_string());
                }
                Err(e) => {
                    self.screen = Screen::WorktreeList { project_index };
                    self.list_index = worktree_index + 1;
                    self.status_message = Some(format!("Failed to delete worktree: {e}"));
                }
            }
        }
        Action::None
    }

    /// Continue from the no-git warning — go straight to CLI tool select or launch.
    pub fn continue_no_git(&mut self) -> Action {
        if let Screen::NoGitWarning { project_index, .. } = self.screen {
            let project = &self.config.projects[project_index];
            let dummy_worktree = WorktreeInfo {
                name: project.name.clone(),
                path: std::path::PathBuf::from(&project.path),
                branch: String::new(),
                is_main: true,
            };

            // Check per-project default CLI tool
            if let Some(ref default_tool) = project.default_tool {
                if let Some(tool_cfg) = self.config.find_tool(default_tool) {
                    return Action::LaunchTool {
                        project: project.clone(),
                        worktree: dummy_worktree,
                        cli: tool_cfg.clone(),
                    };
                }
            }

            // Check global default CLI tool
            if let Some(ref default_tool) = self.config.default_tool {
                if let Some(tool_cfg) = self.config.find_tool(default_tool) {
                    return Action::LaunchTool {
                        project: project.clone(),
                        worktree: dummy_worktree,
                        cli: tool_cfg.clone(),
                    };
                }
            }

            // Show CLI tool selector
            self.screen = Screen::ToolSelect {
                project_index,
                worktree: dummy_worktree,
            };
            self.list_index = 0;
        }
        Action::None
    }

    /// Add a character to the search query and update the filter.
    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
        self.list_index = 0;
    }

    /// Remove the last character from the search query.
    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.update_filter();
        if self.list_index >= self.filtered_indices.len() && !self.filtered_indices.is_empty() {
            self.list_index = self.filtered_indices.len() - 1;
        }
    }

    /// Clear the search query entirely.
    pub fn search_clear(&mut self) {
        self.search_query.clear();
        self.update_filter();
        self.list_index = 0;
    }

    /// Recalculate filtered_indices based on the current search query.
    fn update_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            self.filtered_indices = (0..self.config.projects.len()).collect();
        } else {
            self.filtered_indices = self
                .config
                .projects
                .iter()
                .enumerate()
                .filter(|(_, p)| fuzzy_match(&p.name.to_lowercase(), &query))
                .map(|(i, _)| i)
                .collect();
        }
    }
}

/// Simple fuzzy match: all characters in the query appear in order in the target.
fn fuzzy_match(target: &str, query: &str) -> bool {
    let mut target_chars = target.chars();
    for qc in query.chars() {
        loop {
            match target_chars.next() {
                Some(tc) if tc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}
