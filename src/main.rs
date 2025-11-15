use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    error::Error,
    fs::{copy, create_dir_all},
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use tui_tree_widget::Tree;
use two_face::theme::EmbeddedThemeName;

use crate::app::AppMode;

mod app;

fn get_templates_path() -> PathBuf {
    // Get home directory and build path
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("Could not determine home directory");
    PathBuf::from(home).join(".bundlegem/templates")
}

fn cleanup_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load syntax highlighting resources with extended syntax support
    let ss = two_face::syntax::extra_newlines();
    let theme_set = two_face::theme::extra();
    let theme = &theme_set.get(EmbeddedThemeName::InspiredGithub);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let templates_path = get_templates_path();
    let mut app = app::App::new(&templates_path)?;
    let mut should_exit = false;

    // Main loop with error handling
    let result = (|| -> Result<(), Box<dyn Error>> {
        while !should_exit {
            terminal.draw(|f| {
            match &app.mode {
                AppMode::TreeView => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(3)])
                        .split(f.area());

                    let display_items = app::render_tree_with_checkboxes(&app.items, &app);

                    let tree_widget = Tree::new(&display_items)
                        .expect("Failed to create tree widget")
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!(" Templates: {} ({} selected) ",
                                    templates_path.display(),
                                    app.selected_files.len()
                                ))
                        )
                        .highlight_style(
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        )
                        .highlight_symbol(">> ");

                    f.render_stateful_widget(tree_widget, chunks[0], &mut app.tree_state);

                    let help = Paragraph::new("↑/k: Up | ↓/j: Down | ←/h: Collapse | →/l: Expand/View | Space: Toggle | e: Export | q: Quit")
                        .block(Block::default().borders(Borders::ALL).title(" Help "))
                        .style(Style::default().fg(Color::Gray));

                    f.render_widget(help, chunks[1]);
                }
                AppMode::FileView { path, content, scroll } => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(3)])
                        .split(f.area());

                    let visible_height = chunks[0].height.saturating_sub(2) as usize;
                    let total_lines = content.len();

                    // Build visible content with tildes for lines beyond EOF
                    let mut visible_content: Vec<Line> = Vec::new();
                    for i in 0..visible_height {
                        let line_idx = scroll + i;
                        if line_idx < total_lines {
                            visible_content.push(content[line_idx].clone());
                        } else {
                            // Add tilde for empty lines beyond EOF
                            visible_content.push(Line::from(
                                Span::styled("~", Style::default().fg(Color::DarkGray))
                            ));
                        }
                    }

                    let file_name = PathBuf::from(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&path)
                        .to_string();

                    // Calculate scroll position indicator
                    let position = if total_lines == 0 {
                        "Empty".to_string()
                    } else if *scroll == 0 {
                        "Top".to_string()
                    } else if scroll + visible_height >= total_lines {
                        "Bottom".to_string()
                    } else {
                        let percentage = ((scroll + visible_height / 2) * 100) / total_lines;
                        format!("{}%", percentage)
                    };

                    let paragraph = Paragraph::new(visible_content)
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .title(format!(" Viewing: {} ({} - line {}/{}) ",
                                    file_name,
                                    position,
                                    scroll + 1,
                                    total_lines.max(1)
                                ))
                        );

                    f.render_widget(paragraph, chunks[0]);

                    let help = Paragraph::new("↑/k: Scroll Up | ↓/j: Scroll Down | ←/h: Back to Tree | q/Esc: Back to Tree")
                        .block(Block::default().borders(Borders::ALL).title(" Help "))
                        .style(Style::default().fg(Color::Gray));

                    f.render_widget(help, chunks[1]);
                }
            }
        })?;

            // Poll for events with a small timeout
            if poll(Duration::from_millis(0))? {
                // Drain all pending events and only process the last one
                let mut last_key_event = None;
                while poll(Duration::from_millis(0))? {
                    if let Event::Key(key) = event::read()? {
                        last_key_event = Some(key);
                    }
                }

                if let Some(key) = last_key_event {
                    match &app.mode {
                        AppMode::TreeView => {
                            match key.code {
                                KeyCode::Char('q') => should_exit = true,
                                KeyCode::Char('e') => break,
                                KeyCode::Char('v') | KeyCode::Right | KeyCode::Char('l') => {
                                    app.view_selected_file(&ss, theme)?;
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    app.tree_state.key_down();
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    app.tree_state.key_up();
                                }
                                KeyCode::Left | KeyCode::Char('h') => {
                                    app.handle_left_key();
                                }
                                KeyCode::Char(' ') => app.toggle_selected_file(),
                                _ => {}
                            };
                        }
                        AppMode::FileView { .. } => {
                            let visible_height = terminal.size()?.height.saturating_sub(5) as usize;
                            match key.code {
                                KeyCode::Char('q')
                                | KeyCode::Esc
                                | KeyCode::Left
                                | KeyCode::Char('h') => {
                                    app.exit_file_view();
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    app.scroll_down(visible_height)
                                }
                                KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                                _ => {}
                            };
                        }
                    }
                }
            }
        }

        Ok(())
    })();

    // Always restore terminal, even on error
    cleanup_terminal(&mut terminal)?;

    // Handle the result after terminal is cleaned up
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        return Err(e);
    }

    // Print selected files if user pressed 'e'
    if !should_exit && !app.selected_files.is_empty() {
        println!("\nSelected files:");
        println!("{}", "=".repeat(50));
        let mut sorted_files: Vec<_> = app.selected_files.iter().collect();
        sorted_files.sort();
        for file in sorted_files {
            println!("  • {}", file);
            let src_path = Path::new(file);
            let template_root = get_templates_path();

            // Get relative dest path by stripping prefix template_path from src_path
            if let Ok(dest_path) = src_path
                .strip_prefix(template_root)
                .and_then(|p| p.strip_prefix("template-arduino"))
            {
                // Strip the .tt suffix from our relative dest
                if let Some(dest_string) = dest_path.to_string_lossy().strip_suffix(".tt") {
                    let dest_path = Path::new(dest_string);

                    // Abort if a file already exists at the destinations path
                    if dest_path.exists() {
                        println!(
                            "Skipping copy because file existed locally. {}",
                            dest_path.to_string_lossy()
                        );
                        continue;
                    }

                    // Create parent folders if needed
                    if let Some(parent_dir) = dest_path.parent() {
                        create_dir_all(parent_dir)
                            .expect("error: unable to create folders to import file.");
                    }

                    // Perform file copy
                    println!("About to do copy: {}", src_path.to_string_lossy());
                    copy(src_path, dest_path).expect("error: couldn't copy src to dest");
                }
            }
        }
        println!("{}", "=".repeat(50));
        println!("Total: {} file(s) selected\n", app.selected_files.len());
    } else if !should_exit {
        println!("\nNo files selected.\n");
    }

    Ok(())
}
