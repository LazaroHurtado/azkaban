mod app;
mod config;
mod session;
mod terminal;
mod ui;
mod worktree;

use app::{Action, App, Screen};
use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

fn main() -> Result<()> {
    color_eyre::install()?;

    // Quick debug mode: print config and exit
    if std::env::args().any(|a| a == "--debug") {
        let config = config::Config::load()?;
        println!("Root dir: {:?}", config.root_dir);
        println!("Container: {}", config.container_name);
        println!("project_dirs: {:?}", config.project_dirs);
        println!("CLI tools: {:?}", config.cli_tools.iter().map(|t| &t.name).collect::<Vec<_>>());
        println!("Projects ({}):", config.projects.len());
        for p in &config.projects {
            println!("  - {} -> {}", p.name, p.path);
        }
        return Ok(());
    }

    let config = config::Config::load()?;
    let app = App::new(config);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let result = run_app(&mut term, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;

    result
}

fn run_app(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> Result<()> {
    loop {
        term.draw(|frame| ui::draw(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let action = match &app.screen {
                Screen::NewWorktree { .. } => match key.code {
                    KeyCode::Left => {
                        app.go_back();
                        Action::None
                    }
                    KeyCode::Enter => app.confirm_new_worktree(),
                    KeyCode::Backspace => {
                        app.handle_backspace();
                        Action::None
                    }
                    KeyCode::Esc => Action::Quit,
                    KeyCode::Char(c) => app.handle_char(c),
                    _ => Action::None,
                },
                Screen::ConfirmDelete { .. } => match key.code {
                    KeyCode::Char('y') | KeyCode::Right => app.confirm_delete(),
                    KeyCode::Char('n') | KeyCode::Left => {
                        app.go_back();
                        Action::None
                    }
                    KeyCode::Esc => Action::Quit,
                    _ => Action::None,
                },
                Screen::NoGitWarning { .. } => match key.code {
                    KeyCode::Left | KeyCode::Right => {
                        if let Screen::NoGitWarning { ref mut selected_button, .. } = app.screen {
                            *selected_button = if *selected_button == 0 { 1 } else { 0 };
                        }
                        Action::None
                    }
                    KeyCode::Enter => {
                        if let Screen::NoGitWarning { selected_button, .. } = app.screen {
                            if selected_button == 0 {
                                app.go_back();
                                Action::None
                            } else {
                                app.continue_no_git()
                            }
                        } else {
                            Action::None
                        }
                    }
                    KeyCode::Esc => Action::Quit,
                    _ => Action::None,
                },
                _ => match key.code {
                    KeyCode::Esc => {
                        if matches!(app.screen, Screen::ProjectList) && !app.search_query.is_empty() {
                            app.search_clear();
                            Action::None
                        } else {
                            Action::Quit
                        }
                    }
                    KeyCode::Up => {
                        app.move_up();
                        Action::None
                    }
                    KeyCode::Down => {
                        app.move_down();
                        Action::None
                    }
                    KeyCode::Right => {
                        if matches!(app.screen, Screen::SessionList { .. }) {
                            app.select_session()
                        } else {
                            app.select()
                        }
                    }
                    KeyCode::Left => {
                        app.go_back();
                        Action::None
                    }
                    KeyCode::Delete | KeyCode::Backspace
                        if matches!(app.screen, Screen::WorktreeList { .. }) =>
                    {
                        app.delete_worktree()
                    }
                    KeyCode::Char('r') if !matches!(app.screen, Screen::ProjectList) => Action::Refresh,
                    KeyCode::Char(c) if matches!(app.screen, Screen::ProjectList) => {
                        app.search_push(c);
                        Action::None
                    }
                    KeyCode::Backspace if matches!(app.screen, Screen::ProjectList) => {
                        app.search_pop();
                        Action::None
                    }
                    _ => Action::None,
                },
            };

            match action {
                Action::Quit => return Ok(()),
                Action::LaunchTool {
                    project,
                    worktree,
                    cli,
                } => {
                    // Exit TUI for terminal handoff
                    disable_raw_mode()?;
                    execute!(term.backend_mut(), LeaveAlternateScreen)?;

                    let container_name = &app.config.container_name;
                    if let Err(e) = terminal::launch_tool(container_name, &project, &worktree, &cli) {
                        eprintln!("Error launching CLI tool: {e}");
                        eprintln!("Press Enter to return to Azkaban...");
                        let _ = std::io::stdin().read_line(&mut String::new());
                    }

                    // Re-enter TUI
                    enable_raw_mode()?;
                    execute!(term.backend_mut(), EnterAlternateScreen)?;
                    term.clear()?;

                    app.go_back();
                    if matches!(app.screen, Screen::ToolSelect { .. }) {
                        app.go_back();
                    }
                }
                Action::ResumeSession {
                    project,
                    worktree,
                    cli,
                    session_id,
                } => {
                    disable_raw_mode()?;
                    execute!(term.backend_mut(), LeaveAlternateScreen)?;

                    let container_name = &app.config.container_name;
                    if let Err(e) = terminal::resume_session(container_name, &project, &worktree, &cli, &session_id) {
                        eprintln!("Error resuming session: {e}");
                        eprintln!("Press Enter to return to Azkaban...");
                        let _ = std::io::stdin().read_line(&mut String::new());
                    }

                    enable_raw_mode()?;
                    execute!(term.backend_mut(), EnterAlternateScreen)?;
                    term.clear()?;

                    app.go_back();
                }
                Action::Refresh => {
                    if let Screen::WorktreeList { project_index } = app.screen {
                        let project = &app.config.projects[project_index];
                        let path = std::path::Path::new(&project.path);
                        if let Ok(wts) = worktree::list_worktrees(path) {
                            app.worktrees = wts;
                        }
                    }
                    app.status_message = Some("Refreshed".to_string());
                }
                Action::None => {}
            }
        }
    }
}
