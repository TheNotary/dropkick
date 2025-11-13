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
    widgets::{Block, Borders, Paragraph},
};
use std::{
    collections::HashSet,
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

struct App {
    tree_state: TreeState<String>,
    items: Vec<TreeItem<'static, String>>,
    selected_files: HashSet<String>,
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

            let help = Paragraph::new("↑/k: Up | ↓/j: Down | ←/h: Collapse | →/l: Expand | Space: Toggle | e: Export | q: Quit")
                .block(Block::default().borders(Borders::ALL).title(" Help "))
                .style(Style::default().fg(Color::Gray));

            f.render_widget(help, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => should_exit = true,
                KeyCode::Char('e') => break,
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
