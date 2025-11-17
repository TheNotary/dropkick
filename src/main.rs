use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use regex::Regex;
use std::{
    error::Error,
    fs::{copy, create_dir_all},
    io,
    path::{Path, PathBuf},
    time::Duration,
};

use two_face::theme::EmbeddedThemeName;

use crate::{app::Action, template_rendering::render_template};

mod app;
mod interpolation_config;
mod template_rendering;

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
            terminal.draw(|f| app.render(f))?;

            ////////////////////////
            // Handle User Inputs //
            ////////////////////////
            if poll(Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    let action = app.handle_key(key, &terminal, &ss, theme)?;
                    match action {
                        Action::Quit => should_exit = true,
                        Action::Extract => {
                            break;
                        }
                        Action::Continue => {}
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
        println!("\nSelected template files imported:");
        println!("{}", "=".repeat(50));
        let mut n_imports = 0;
        let mut sorted_files: Vec<_> = app.selected_files.iter().collect();
        sorted_files.sort();
        for file in sorted_files {
            let src_path = Path::new(file);

            n_imports += import_selected_template_file(src_path).is_some() as u32;
        }

        // Print Summary
        println!("{}", "=".repeat(50));
        println!(
            "Imported: {} of {} selected file(s)\n",
            n_imports,
            app.selected_files.len()
        );
    } else if !should_exit {
        println!("\nNo files selected.\n");
    }

    let my_template = "
Hello #{config[:name]}!
k8s stuff: #{config[:k8s_domain]}
";

    let re = Regex::new(r"#\{config\[:(\w+)\]\}").unwrap();
    let my_template = re.replace_all(my_template, "{$1}").to_string();

    render_template(&my_template).expect("error template rendering");

    Ok(())
}

fn get_templates_path() -> PathBuf {
    get_home().join(".dropkick/templates")
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

fn import_selected_template_file(src_path: &Path) -> Option<u8> {
    let template_root = get_templates_path();

    // Compute relative destination, and create a PathBuff, since we need
    // to mutate it, we can't just have it be an &Path???
    let mut dest = src_path.strip_prefix(template_root).ok()?.to_path_buf();

    // Remove the first segment (template folder)
    dest = dest.iter().skip(1).collect::<PathBuf>();

    // Remove `.tt` suffix
    dest = dest.with_extension("");

    // Abort if a file already exists
    if dest.exists() {
        println!(
            "Skipping copy of '{}' because file existed locally.",
            dest.to_string_lossy()
        );
        return None;
    }

    // Create parent directories
    if let Some(parent) = dest.parent() {
        let display_path = clean_path(src_path);
        create_dir_all(parent).expect("error: unable to create parent directories.");
        println!("  • {}", display_path.to_string_lossy());
    }

    // Copy file
    copy(src_path, &dest).expect("error: couldn't copy src to dest");

    Some(1)
}

fn get_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .expect("Could not determine home directory")
}

fn clean_path(src_path: &Path) -> PathBuf {
    let home = get_home();
    src_path
        .strip_prefix(&home)
        .map(|p| PathBuf::from("~").join(p))
        .unwrap_or_else(|_| src_path.to_path_buf())
}
