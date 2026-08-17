mod app;
mod ui;

use anyhow::Result;
use app::App;
use crossterm::event::{self, Event};
use std::{path::PathBuf, time::Duration};

#[derive(Debug, Default)]
pub struct TuiOutcome {
    pub stdout: Vec<String>,
}

pub fn run(root: PathBuf) -> Result<TuiOutcome> {
    let mut app = App::new(root)?;
    let mut terminal = ratatui::try_init()?;

    let loop_result = run_loop(&mut terminal, &mut app);
    let restore_result = ratatui::try_restore();

    loop_result?;
    restore_result?;
    Ok(TuiOutcome {
        stdout: app.stdout_after_exit,
    })
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;
        if event::poll(Duration::from_millis(120))? {
            if let Event::Key(key) = event::read()? {
                app.handle_key(key)?;
            }
        }
    }
    Ok(())
}
