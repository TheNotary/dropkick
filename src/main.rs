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
    collections::HashSet,
    error::Error,
    fs::{self, copy},
    io,
    path::{Path, PathBuf},
    time::Duration,
};
use syntect::{
    easy::HighlightLines,
    highlighting::{Style as SyntectStyle, Theme},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};
use tui_tree_widget::{Tree, TreeItem, TreeState};
use two_face::theme::EmbeddedThemeName;

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

        // Open and select the first item by default
        if !items.is_empty() {
            let first_id = items[0].identifier().clone();
            tree_state.open(vec![first_id.clone()]);
            tree_state.select(vec![first_id]);
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

    fn view_selected_file(&mut self, ss: &SyntaxSet, theme: &Theme) -> Result<(), Box<dyn Error>> {
        if let Some(selected) = self.tree_state.selected().last() {
            let path = PathBuf::from(selected);
            if path.is_file() {
                // Try to read as UTF-8, skip if binary
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        let highlighted = highlight_file(&content, &path, ss, theme)?;

                        self.mode = AppMode::FileView {
                            path: selected.clone(),
                            content: highlighted,
                            scroll: 0,
                        };
                    }
                    Err(_) => {
                        // File is likely binary, just stay in tree view
                        // Could optionally show an error message here
                    }
                }
            } else if path.is_dir() {
                // For directories, just expand them
                self.tree_state.key_right();
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

    fn handle_left_key(&mut self) {
        self.tree_state.key_left();

        // If selection is empty after left, re-select the first item
        if self.tree_state.selected().is_empty() && !self.items.is_empty() {
            let first_id = self.items[0].identifier().clone();
            self.tree_state.select(vec![first_id]);
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

fn get_syntax_for_special_file<'a>(
    path: &Path,
    ss: &'a SyntaxSet,
) -> Option<&'a syntect::parsing::SyntaxReference> {
    // Handle files without extensions by name
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        // Strip .tt if present to get the actual filename
        let name = file_name.strip_suffix(".tt").unwrap_or(file_name);

        match name.to_lowercase().as_str() {
            "dockerfile" => {
                // Try multiple possible names/extensions for Docker
                ss.find_syntax_by_name("Docker")
                    .or_else(|| ss.find_syntax_by_extension("dockerfile"))
                    .or_else(|| ss.find_syntax_by_name("Dockerfile"))
            }
            "gemfile" | "rakefile" | "guardfile" | "capfile" | "vagrantfile" => {
                ss.find_syntax_by_name("Ruby")
            }
            "makefile" => ss.find_syntax_by_name("Makefile"),
            "cmakelists.txt" => ss.find_syntax_by_name("CMake"),
            "justfile" => ss.find_syntax_by_name("Just"),
            _ => None,
        }
    } else {
        None
    }
}

fn highlight_file(
    content: &str,
    path: &Path,
    ss: &SyntaxSet,
    theme: &Theme,
) -> Result<Vec<Line<'static>>, Box<dyn Error>> {
    // Determine the syntax based on file extension or name
    let syntax = if let Some(syntax) = get_syntax_for_special_file(path, ss) {
        syntax
    } else if path.extension().and_then(|e| e.to_str()) == Some("tt") {
        // For .tt files, strip the .tt and get syntax from the underlying extension
        let path_str = path.to_string_lossy();
        if let Some(stripped) = path_str.strip_suffix(".tt") {
            let underlying_path = Path::new::<str>(stripped.as_ref());
            // Use find_syntax_by_extension which is safer (doesn't do IO)
            if let Some(ext) = underlying_path.extension().and_then(|e| e.to_str()) {
                ss.find_syntax_by_extension(ext)
                    .unwrap_or_else(|| ss.find_syntax_plain_text())
            } else {
                ss.find_syntax_plain_text()
            }
        } else {
            // if the file path string doesn't have the .tt extension
            ss.find_syntax_plain_text() // wait, I will not get here ever?
        }
    } else {
        // For non-.tt files, use the extension directly
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            ss.find_syntax_by_extension(ext)
                .unwrap_or_else(|| ss.find_syntax_plain_text())
        } else {
            ss.find_syntax_plain_text()
        }
    };

    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut lines = Vec::new();

    for line in LinesWithEndings::from(content) {
        let ranges: Vec<(SyntectStyle, &str)> = highlighter.highlight_line(line, ss)?;

        let spans: Vec<Span> = ranges
            .into_iter()
            .map(|(style, text)| {
                let text_expanded = text.replace('\t', "  ");
                Span::styled(
                    text_expanded,
                    Style::default().fg(syntect_to_ratatui_color(style.foreground)),
                )
            })
            .collect();

        lines.push(Line::from(spans));
    }

    Ok(lines)
}

fn should_show_entry(path: &Path) -> bool {
    // Get the file name
    let file_name = match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => name,
        None => return false,
    };

    // Hide .DS_Store files
    if file_name.eq_ignore_ascii_case(".ds_store") {
        return false;
    }

    if file_name.eq_ignore_ascii_case(".git") {
        return false;
    }

    if file_name.eq_ignore_ascii_case("node_modules") {
        return false;
    }

    // Always show directories
    if path.is_dir() {
        return true;
    }

    // For files, only show .tt files
    if path.is_file() {
        return file_name.ends_with(".tt");
    }

    false
}

fn build_tree(path: &Path) -> Result<Vec<TreeItem<'static, String>>, Box<dyn Error>> {
    let mut items = Vec::new();

    if !path.exists() {
        return Ok(items);
    }

    let entries = fs::read_dir(path)?;
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| should_show_entry(p))
        .collect();

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
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| should_show_entry(p))
        .collect();

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
    let file_name = PathBuf::from(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string();

    // Strip .tt extension for display
    file_name
        .strip_suffix(".tt")
        .unwrap_or(&file_name)
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

    // InspiredGitHub

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let templates_path = get_templates_path();

    // Create app state
    let mut app = App::new(&templates_path)?;
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
            // CHECK TODO: Do not allow overwriting existing files
            // TODO: Make sure you copy files relative to the root template
            let src_path = Path::new(file);
            if let Some(dest_path) = src_path.file_name() {
                if let Some(dest_path) = dest_path.to_string_lossy().strip_suffix(".tt") {
                    if Path::new(dest_path).exists() {
                        println!("Skipping copy because file existed locally. {}", dest_path);
                        continue;
                    }
                    copy(src_path, Path::new(dest_path))?;
                }
            }
        }
        println!("{}", "=".repeat(50));
        println!("Total: {} file(s) selected\n", app.selected_files.len());
        // TODO: Map over the files, and then copy them into the working directory
    } else if !should_exit {
        println!("\nNo files selected.\n");
    }

    Ok(())
}
