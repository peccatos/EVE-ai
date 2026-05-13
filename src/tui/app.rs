use std::io::{self, IsTerminal, Write};

use crate::tui::events::{parse_event, TuiEvent};
use crate::tui::state::load_tui_state;
use crate::tui::ui::{render_screen, TuiScreen};

pub fn run_tui(project_root: &str, memory_root: &str) -> Result<String, String> {
    let mut state = load_tui_state(project_root, memory_root);
    let mut screen = TuiScreen::Dashboard;
    let width = std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);

    if !io::stdin().is_terminal() {
        return Ok(render_screen(&state, screen, width));
    }

    loop {
        print!("\x1b[2J\x1b[H{}", render_screen(&state, screen, width));
        print!("\ncommand> ");
        io::stdout()
            .flush()
            .map_err(|error| format!("failed to flush tui: {error}"))?;
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|error| format!("failed to read tui input: {error}"))?;
        match parse_event(&input) {
            TuiEvent::Quit => return Ok("tui_status: closed".to_string()),
            TuiEvent::Refresh => state = load_tui_state(project_root, memory_root),
            TuiEvent::Switch(next) => screen = next,
            TuiEvent::None => {}
        }
    }
}
