use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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
    collections::HashSet,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

enum AppMode {
    TreeView,
    FileView {
        path: String,
        content: Vec<Line<'static>>,
        scroll: usize,
    },
}

struct App {
    tree_state: TreeState<String>,
    items: Vec<TreeItem<'static, String>>,
    selected_files: HashSet<String>,
    mode: AppMode,
}

impl App {
    fn new(root_path: &Path) -> Result<Self, Box<dyn Error>> {
        let items = build_tree(root_path)?;
        let mut tree_state = TreeState::default();

        // Open the first item by default
        if !items.is_empty() {
            tree_state.open(vec![items[0].identifier().clone()]);
        }

        Ok(Self {
            tree_state,
            items,
            selected_files: HashSet::new(),
            mode: AppMode::TreeView,
        })
    }

    fn toggle_selected_file(&mut self) {
        if let Some(selected) = self.tree_state.selected().last() {
            let path = PathBuf::from(selected);
            if path.is_file() {
                if self.selected_files.contains(selected) {
                    self.selected_files.remove(selected);
                } else {
                    self.selected_files.insert(selected.clone());
                }
            }
        }
    }

    fn view_selected_file(&mut self, ss: &SyntaxSet, ts: &ThemeSet) -> Result<(), Box<dyn Error>> {
        if let Some(selected) = self.tree_state.selected().last() {
            let path = PathBuf::from(selected);
            if path.is_file() {
                let content = fs::read_to_string(&path)?;
                let highlighted = highlight_file(&content, &path, ss, ts)?;

                self.mode = AppMode::FileView {
                    path: selected.clone(),
                    content: highlighted,
                    scroll: 0,
                };
            }
        }
        Ok(())
    }

    fn scroll_up(&mut self) {
        if let AppMode::FileView { scroll, .. } = &mut self.mode {
            *scroll = scroll.saturating_sub(1);
        }
    }

    fn scroll_down(&mut self, max_lines: usize) {
        if let AppMode::FileView {
            scroll, content, ..
        } = &mut self.mode
        {
            if *scroll + max_lines < content.len() {
                *scroll += 1;
            }
        }
    }

    fn exit_file_view(&mut self) {
        self.mode = AppMode::TreeView;
    }

    fn get_display_text(&self, identifier: &str, text: &str) -> String {
        let path = PathBuf::from(identifier);
        if path.is_file() {
            let checkbox = if self.selected_files.contains(identifier) {
                "[x]"
            } else {
                "[ ]"
            };
            format!("{} {}", checkbox, text)
        } else {
            text.to_string()
        }
    }
}

fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

fn highlight_file(
    content: &str,
    path: &Path,
    ss: &SyntaxSet,
    ts: &ThemeSet,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    let syntax = ss
        .find_syntax_for_file(path)?
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = &ts.themes["base16-ocean.dark"];
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut lines = Vec::new();

    for line in LinesWithEndings::from(content) {
        let ranges: Vec<(SyntectStyle, &str)> = highlighter.highlight_line(line, ss)?;

        let spans: Vec<Span> = ranges
            .into_iter()
            .map(|(style, text)| {
                Span::styled(
                    text.to_string(),
                    Style::default().fg(syntect_to_ratatui_color(style.foreground)),
                )
            })
            .collect();

        lines.push(Line::from(spans));
    }

    Ok(lines)
}

fn build_tree(path: &Path) -> Result<Vec<TreeItem<'static, String>>, Box<dyn Error>> {
    let mut items = Vec::new();

    if !path.exists() {
        return Ok(items);
    }

    let entries = fs::read_dir(path)?;
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();

    paths.sort();

    for entry in paths {
        if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
            let identifier = entry.to_string_lossy().to_string();

            if entry.is_dir() {
                let children = build_tree_recursive(&entry)?;
                items.push(TreeItem::new(identifier, name.to_string(), children)?);
            } else {
                items.push(TreeItem::new_leaf(identifier, name.to_string()));
            }
        }
    }

    Ok(items)
}

fn build_tree_recursive(path: &Path) -> Result<Vec<TreeItem<'static, String>>, Box<dyn Error>> {
    let mut items = Vec::new();

    let entries = fs::read_dir(path)?;
    let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();

    paths.sort();

    for entry in paths {
        if let Some(name) = entry.file_name().and_then(|n| n.to_str()) {
            let identifier = entry.to_string_lossy().to_string();

            if entry.is_dir() {
                let children = build_tree_recursive(&entry)?;
                items.push(TreeItem::new(identifier, name.to_string(), children)?);
            } else {
                items.push(TreeItem::new_leaf(identifier, name.to_string()));
            }
        }
    }

    Ok(items)
}

fn get_item_text(path: &str) -> String {
    PathBuf::from(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string()
}

fn render_tree_with_checkboxes<'a>(
    items: &'a [TreeItem<'a, String>],
    app: &App,
) -> Vec<TreeItem<'a, String>> {
    items
        .iter()
        .map(|item| {
            let text = get_item_text(item.identifier());
            let display_text = app.get_display_text(item.identifier(), &text);

            if item.children().is_empty() {
                TreeItem::new_leaf(item.identifier().clone(), display_text)
            } else {
                let children = render_tree_with_checkboxes(item.children(), app);
                TreeItem::new(item.identifier().clone(), display_text, children)
                    .expect("Failed to create tree item")
            }
        })
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load syntax highlighting resources
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Get home directory and build path
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("Could not determine home directory");
    let templates_path = PathBuf::from(home).join(".bundlegem/templates");

    // Create app state
    let mut app = App::new(&templates_path)?;
    let mut should_exit = false;

    // Main loop
    while !should_exit {
        terminal.draw(|f| {
            match &app.mode {
                AppMode::TreeView => {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Min(0), Constraint::Length(3)])
                        .split(f.area());

                    let display_items = render_tree_with_checkboxes(&app.items, &app);

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

                    let help = Paragraph::new("↑/k: Up | ↓/j: Down | ←/h: Collapse | →/l: Expand | Space: Toggle | v: View | e: Export | q: Quit")
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
                        .unwrap_or(path)
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

                    let help = Paragraph::new("↑/k: Scroll Up | ↓/j: Scroll Down | q/Esc: Back to Tree")
                        .block(Block::default().borders(Borders::ALL).title(" Help "))
                        .style(Style::default().fg(Color::Gray));

                    f.render_widget(help, chunks[1]);
                }
            }
        })?;

        if let Event::Key(key) = event::read()? {
            match &app.mode {
                AppMode::TreeView => {
                    match key.code {
                        KeyCode::Char('q') => should_exit = true,
                        KeyCode::Char('e') => break,
                        KeyCode::Char('v') => app.view_selected_file(&ss, &ts)?,
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.tree_state.key_down();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.tree_state.key_up();
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            app.tree_state.key_left();
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            app.tree_state.key_right();
                        }
                        KeyCode::Char(' ') => app.toggle_selected_file(),
                        _ => {}
                    };
                }
                AppMode::FileView { .. } => {
                    let visible_height = terminal.size()?.height.saturating_sub(5) as usize;
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => app.exit_file_view(),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(visible_height),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                        _ => {}
                    };
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Print selected files if user pressed 'e'
    if !should_exit && !app.selected_files.is_empty() {
        println!("\nSelected files:");
        println!("{}", "=".repeat(50));
        let mut sorted_files: Vec<_> = app.selected_files.iter().collect();
        sorted_files.sort();
        for file in sorted_files {
            println!("  • {}", file);
        }
        println!("{}", "=".repeat(50));
        println!("Total: {} file(s) selected\n", app.selected_files.len());
    } else if !should_exit {
        println!("\nNo files selected.\n");
    }

    Ok(())
}
