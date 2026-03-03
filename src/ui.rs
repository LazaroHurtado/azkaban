use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::app::{App, Screen};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title bar
            Constraint::Min(5),   // Main content
            Constraint::Length(3), // Status / help bar
        ])
        .split(frame.area());

    draw_title_bar(frame, app, chunks[0]);
    draw_main_content(frame, app, chunks[1]);
    draw_status_bar(frame, app, chunks[2]);
}

fn draw_title_bar(frame: &mut Frame, _app: &App, area: Rect) {
    let title = Line::from(vec![
        Span::styled(
            " Azkaban ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(title).block(block);
    frame.render_widget(paragraph, area);
}

fn draw_main_content(frame: &mut Frame, app: &App, area: Rect) {
    match &app.screen {
        Screen::ProjectList => draw_project_list(frame, app, area, true),
        Screen::WorktreeList { project_index } => {
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(area);
            draw_project_list(frame, app, panes[0], false);
            draw_worktree_list(frame, app, *project_index, panes[1], true);
        }
        Screen::SessionList {
            project_index,
            worktree_index,
        } => {
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(25),
                    Constraint::Percentage(30),
                    Constraint::Percentage(45),
                ])
                .split(area);
            draw_project_list(frame, app, panes[0], false);
            draw_worktree_list(frame, app, *project_index, panes[1], false);
            draw_session_list(frame, app, *worktree_index, panes[2]);
        }
        Screen::ToolSelect {
            project_index,
            worktree,
        } => draw_tool_select(frame, app, *project_index, &worktree.name, area),
        Screen::NewWorktree {
            project_index,
            input,
        } => draw_new_worktree(frame, app, *project_index, input, area),
        Screen::ConfirmDelete {
            worktree_index, ..
        } => draw_confirm_delete(frame, app, *worktree_index, area),
        Screen::NoGitWarning {
            project_index, ..
        } => draw_no_git_warning(frame, app, *project_index, area),
    }
}

fn draw_project_list(frame: &mut Frame, app: &App, area: Rect, focused: bool) {
    let selected_index = if focused {
        app.list_index
    } else {
        app.project_list_index
    };

    let show_search = focused && !app.search_query.is_empty();

    // Split area: main list + optional search bar at bottom
    let list_area = if show_search {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);

        // Draw search bar
        let search_bar = Paragraph::new(Line::from(vec![
            Span::styled(" 🔍 ", Style::default().fg(Color::Yellow)),
            Span::styled(&app.search_query, Style::default().fg(Color::White)),
            Span::styled("█", Style::default().fg(Color::DarkGray)),
        ]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        frame.render_widget(search_bar, chunks[1]);
        chunks[0]
    } else {
        area
    };

    // Build items from filtered indices when focused, all projects when not
    let indices: Vec<usize> = if focused {
        app.filtered_indices.clone()
    } else {
        (0..app.config.projects.len()).collect()
    };

    let items: Vec<ListItem> = indices
        .iter()
        .enumerate()
        .map(|(display_i, &proj_i)| {
            let p = &app.config.projects[proj_i];
            let is_selected = if focused {
                display_i == selected_index
            } else {
                proj_i == app.filtered_indices.get(app.project_list_index).copied().unwrap_or(usize::MAX)
            };
            let style = if is_selected {
                if focused {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)
                }
            } else if focused {
                Style::default()
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let prefix = if is_selected { "▶ " } else { "  " };
            let tool_hint = p
                .default_tool
                .as_deref()
                .map(|l| format!(" [{l}]"))
                .unwrap_or_default();
            ListItem::new(Line::from(vec![
                Span::styled(format!("{prefix}{}", p.name), style),
                Span::styled(tool_hint, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    if items.is_empty() {
        let msg = if app.search_query.is_empty() {
            "  No projects configured.\n  Edit config.toml in the azkaban repo to add projects."
        } else {
            "  No matching projects."
        };
        let empty_msg = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .title(" Projects ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(empty_msg, list_area);
        return;
    }

    let border_color = if focused { Color::Cyan } else { Color::DarkGray };
    let title_style = if focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = if show_search {
        format!(" Projects ({}/{}) ", indices.len(), app.config.projects.len())
    } else {
        " Projects ".to_string()
    };

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );

    let mut state = ListState::default();
    state.select(Some(selected_index));
    frame.render_stateful_widget(list, list_area, &mut state);
}

fn draw_worktree_list(frame: &mut Frame, app: &App, project_index: usize, area: Rect, focused: bool) {
    let project_name = &app.config.projects[project_index].name;
    let selected_index = if focused {
        app.list_index
    } else {
        app.worktree_list_index
    };

    let mut items: Vec<ListItem> = Vec::new();

    // First item: create new worktree
    let is_new_selected = selected_index == 0;
    let new_style = if is_new_selected {
        if focused {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)
        }
    } else if focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let new_prefix = if is_new_selected { "▶ " } else { "  " };
    items.push(ListItem::new(Span::styled(
        format!("{new_prefix}[+ New Worktree]"),
        new_style,
    )));

    // Existing worktrees
    for (i, wt) in app.worktrees.iter().enumerate() {
        let item_index = i + 1;
        let is_selected = item_index == selected_index;
        let style = if is_selected {
            if focused {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)
            }
        } else if focused {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        let main_marker = if wt.is_main { " (main)" } else { "" };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{prefix}{}", wt.branch), style),
            Span::styled(
                format!("{main_marker}"),
                Style::default().fg(Color::DarkGray),
            ),
        ])));
    }

    let border_color = if focused { Color::Cyan } else { Color::DarkGray };
    let title_style = if focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                format!(" Worktrees — {project_name} "),
                title_style,
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );

    let mut state = ListState::default();
    state.select(Some(selected_index));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_tool_select(
    frame: &mut Frame,
    app: &App,
    _project_index: usize,
    worktree_name: &str,
    area: Rect,
) {
    let items: Vec<ListItem> = app
        .config
        .cli_tools
        .iter()
        .enumerate()
        .map(|(i, cli_tool)| {
            let style = if i == app.list_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if i == app.list_index { "▶ " } else { "  " };
            ListItem::new(Span::styled(
                format!("{prefix}{}", cli_tool.display()),
                style,
            ))
        })
        .collect();

    // Draw as a centered popup
    let popup_area = centered_rect(40, 40, area);
    frame.render_widget(Clear, popup_area);

    let list = List::new(items).block(
        Block::default()
            .title(format!(" Select CLI Tool — {worktree_name} "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    let mut state = ListState::default();
    state.select(Some(app.list_index));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_new_worktree(
    frame: &mut Frame,
    _app: &App,
    project_index: usize,
    input: &str,
    area: Rect,
) {
    // Draw a centered input popup
    let popup_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" New Worktree — Project {} ", project_index))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let text = vec![
        Line::from(Span::styled(
            "Enter branch name:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  > {input}█"),
            Style::default().fg(Color::White),
        )),
    ];
    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, inner);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match &app.screen {
        Screen::ProjectList => if app.search_query.is_empty() {
            "↑/↓ Navigate  → Select  Type to search  Esc Quit"
        } else {
            "↑/↓ Navigate  → Select  Backspace Delete  Esc Clear search"
        },
        Screen::WorktreeList { .. } => "↑/↓ Navigate  → Select  ← Back  Del Delete  Esc Quit",
        Screen::SessionList { .. } => "↑/↓ Navigate  → Resume/New  ← Back  Esc Quit",
        Screen::ToolSelect { .. } => "↑/↓ Navigate  → Select  ← Back  Esc Quit",
        Screen::NewWorktree { .. } => "Type branch name  Enter Confirm  ← Back  Esc Quit",
        Screen::ConfirmDelete { .. } => "y/→ Confirm  n/← Cancel  Esc Quit",
        Screen::NoGitWarning { .. } => "←/→ Select  Enter Confirm  Esc Quit",
    };

    let status = if let Some(ref msg) = app.status_message {
        Line::from(vec![
            Span::styled(format!(" {msg} "), Style::default().fg(Color::Yellow)),
            Span::raw(" │ "),
            Span::styled(help_text, Style::default().fg(Color::DarkGray)),
        ])
    } else {
        Line::from(Span::styled(
            format!(" {help_text}"),
            Style::default().fg(Color::DarkGray),
        ))
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(status).block(block);
    frame.render_widget(paragraph, area);
}

/// Create a centered rectangle of a given percentage of the parent area.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_confirm_delete(frame: &mut Frame, app: &App, worktree_index: usize, area: Rect) {
    let wt_name = &app.worktrees[worktree_index].branch;

    let popup_area = centered_rect(50, 20, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Delete Worktree ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Delete worktree '{wt_name}'?"),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  y ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::styled("Yes  ", Style::default().fg(Color::DarkGray)),
            Span::styled("n ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled("No", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, inner);
}

fn draw_no_git_warning(frame: &mut Frame, app: &App, project_index: usize, area: Rect) {
    let project_name = &app.config.projects[project_index].name;
    let selected_button = if let Screen::NoGitWarning { selected_button, .. } = &app.screen {
        *selected_button
    } else {
        0
    };

    let popup_area = centered_rect(60, 40, area);
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" No Git Repository ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let return_style = if selected_button == 0 {
        Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let continue_style = if selected_button == 1 {
        Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  '{project_name}' is not a git repository."),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Worktree management is not available.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  You can still launch a CLI tool in the project root.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("       "),
            Span::styled(" Return ", return_style),
            Span::raw("   "),
            Span::styled(" Continue ", continue_style),
        ]),
    ];
    let paragraph = Paragraph::new(text);
    frame.render_widget(paragraph, inner);
}

fn draw_session_list(frame: &mut Frame, app: &App, worktree_index: usize, area: Rect) {
    let wt_name = &app.worktrees[worktree_index].branch;

    let mut items: Vec<ListItem> = Vec::new();

    // First item: new session
    let is_new_selected = app.list_index == 0;
    let new_style = if is_new_selected {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let new_prefix = if is_new_selected { "▶ " } else { "  " };
    items.push(ListItem::new(Span::styled(
        format!("{new_prefix}[+ New Session]"),
        new_style,
    )));

    // Existing sessions
    for (i, session) in app.sessions.iter().enumerate() {
        let item_index = i + 1;
        let is_selected = item_index == app.list_index;
        let style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let prefix = if is_selected { "▶ " } else { "  " };
        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{prefix}{}", session.summary), style),
            Span::styled(
                format!("  {}", session.time_ago()),
                Style::default().fg(Color::DarkGray),
            ),
        ])));
    }

    let list = List::new(items).block(
        Block::default()
            .title(Span::styled(
                format!(" Sessions — {} ", wt_name),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    let mut state = ListState::default();
    state.select(Some(app.list_index));
    frame.render_stateful_widget(list, area, &mut state);
}
