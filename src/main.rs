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
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

struct App {
    tree_state: TreeState<String>,
    items: Vec<TreeItem<'static, String>>,
}

impl App {
    fn new(root_path: &Path) -> Result<Self, Box<dyn Error>> {
        let items = build_tree(root_path)?;
        let mut tree_state = TreeState::default();

        // Open the first item by default
        if !items.is_empty() {
            tree_state.open(vec![items[0].identifier().clone()]);
        }

        Ok(Self { tree_state, items })
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

    // Main loop
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(3)])
                .split(f.area());

            let tree_widget = Tree::new(&app.items)
                .expect("Failed to create tree widget")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Templates: {} ", templates_path.display())),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(tree_widget, chunks[0], &mut app.tree_state);

            let help =
                Paragraph::new("↑/↓: Navigate | ←/→: Collapse/Expand | Space: Toggle | q: Quit")
                    .block(Block::default().borders(Borders::ALL).title(" Help "))
                    .style(Style::default().fg(Color::Gray));

            f.render_widget(help, chunks[1]);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') => break,
                KeyCode::Down | KeyCode::Char('j') => app.tree_state.key_down(),
                KeyCode::Up | KeyCode::Char('k') => app.tree_state.key_up(),
                KeyCode::Left | KeyCode::Char('h') => app.tree_state.key_left(),
                KeyCode::Right | KeyCode::Char('l') => app.tree_state.key_right(),
                KeyCode::Char(' ') => app.tree_state.toggle_selected(),
                _ => false,
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

    Ok(())
}
