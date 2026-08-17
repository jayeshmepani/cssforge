use anyhow::{Context, Result};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use cssforge_core::{
    OutputMode, OutputOptions, PlanEntry, Preset, RuleDefinition, RuleId, Safety, WorkspaceReport,
    WriteResult, analyze_workspace, apply_selected_plans, discover_css_files, is_git_dirty,
    rule_definitions, unified_diff, write_result,
};
use std::{fs, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Files,
    Rules,
    Output,
    Done,
    Diff,
}

impl Screen {
    pub const STEPS: [Screen; 4] = [Screen::Files, Screen::Rules, Screen::Output, Screen::Done];

    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Files => "1: Select Files",
            Self::Rules => "2: Select Rules",
            Self::Output => "3: Output Settings",
            Self::Done => "4: Done",
            Self::Diff => "Diff Inspector",
        }
    }

    pub const fn step_index(self) -> Option<usize> {
        match self {
            Self::Files => Some(0),
            Self::Rules => Some(1),
            Self::Output => Some(2),
            Self::Done => Some(3),
            Self::Diff => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileItem {
    pub path: PathBuf,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct RuleItem {
    pub definition: RuleDefinition,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct App {
    pub root: PathBuf,
    pub files: Vec<FileItem>,
    pub rules: Vec<RuleItem>,
    pub preset: Preset,
    pub output_mode: OutputMode,
    pub output_cursor: usize,
    pub report: Option<WorkspaceReport>,
    pub screen: Screen,
    pub file_cursor: usize,
    pub rule_cursor: usize,
    pub plan_cursor: usize,
    pub diff_scroll: u16,
    pub diff_text: String,
    pub status: String,
    pub git_dirty: bool,
    pub should_quit: bool,
    pub stdout_after_exit: Vec<String>,
    pub write_results: Vec<WriteResult>,
}

impl App {
    pub fn new(root: PathBuf) -> Result<Self> {
        let preset = Preset::Modern;
        let discovered = discover_css_files(&root)?;
        let files = discovered
            .into_iter()
            .map(|path| FileItem {
                path,
                selected: true,
            })
            .collect();
        let enabled = preset.enabled_rules();
        let rules = rule_definitions()
            .into_iter()
            .map(|definition| RuleItem {
                enabled: enabled.contains(&definition.id),
                definition,
            })
            .collect();
        let git_dirty = is_git_dirty(&root);
        let output_mode = OutputMode::NewFile;
        let output_cursor = OutputMode::ALL
            .iter()
            .position(|m| *m == output_mode)
            .unwrap_or(1);

        let mut app = Self {
            root,
            files,
            rules,
            preset,
            output_mode,
            output_cursor,
            report: None,
            screen: Screen::Files,
            file_cursor: 0,
            rule_cursor: 0,
            plan_cursor: 0,
            diff_scroll: 0,
            diff_text: String::new(),
            status: "Select CSS files to modernize, then press [Enter] to continue.".to_string(),
            git_dirty,
            should_quit: false,
            stdout_after_exit: Vec::new(),
            write_results: Vec::new(),
        };
        app.analyze()?;
        Ok(app)
    }

    pub fn enabled_rules(&self) -> Vec<RuleId> {
        self.rules
            .iter()
            .filter(|item| item.enabled)
            .map(|item| item.definition.id)
            .collect()
    }

    pub fn selected_files(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.path.clone())
            .collect()
    }

    pub fn analyze(&mut self) -> Result<()> {
        let files = self.selected_files();
        let rules = self.enabled_rules();
        if files.is_empty() || rules.is_empty() {
            self.report = None;
            return Ok(());
        }
        self.report = Some(analyze_workspace(&self.root, &files, &rules)?);
        self.plan_cursor = 0;
        self.diff_scroll = 0;
        self.refresh_diff()?;
        Ok(())
    }

    pub fn refresh_files(&mut self) -> Result<()> {
        let old_selected: std::collections::HashSet<PathBuf> = self
            .files
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.path.clone())
            .collect();
        self.files = discover_css_files(&self.root)?
            .into_iter()
            .map(|path| FileItem {
                selected: old_selected.is_empty() || old_selected.contains(&path),
                path,
            })
            .collect();
        self.file_cursor = self.file_cursor.min(self.files.len().saturating_sub(1));
        self.git_dirty = is_git_dirty(&self.root);
        self.analyze()
    }

    pub fn plan_positions(&self) -> Vec<(usize, usize)> {
        self.report
            .as_ref()
            .map(|report| {
                report
                    .files
                    .iter()
                    .enumerate()
                    .flat_map(|(fi, file)| (0..file.plans.len()).map(move |pi| (fi, pi)))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn current_plan(&self) -> Option<&PlanEntry> {
        let positions = self.plan_positions();
        let (fi, pi) = *positions.get(self.plan_cursor)?;
        self.report.as_ref()?.files.get(fi)?.plans.get(pi)
    }

    fn current_plan_mut(&mut self) -> Option<&mut PlanEntry> {
        let positions = self.plan_positions();
        let (fi, pi) = *positions.get(self.plan_cursor)?;
        self.report.as_mut()?.files.get_mut(fi)?.plans.get_mut(pi)
    }

    pub fn toggle_current_plan(&mut self) -> Result<()> {
        if let Some(plan) = self.current_plan_mut() {
            if matches!(plan.safety, Safety::Safe | Safety::Review) {
                plan.selected = !plan.selected;
            }
        }
        self.refresh_diff()
    }

    pub fn refresh_diff(&mut self) -> Result<()> {
        self.diff_text = if let Some(plan) = self.current_plan() {
            unified_diff(
                &plan.original,
                &plan.proposed,
                &format!("{}:before", plan.file.display()),
                &format!("{}:after", plan.file.display()),
            )
        } else {
            "No transformation plan selected.".to_string()
        };
        Ok(())
    }

    pub fn apply(&mut self) -> Result<bool> {
        self.analyze()?;
        let Some(report) = self.report.clone() else {
            self.status = "Run analysis before applying transformations.".into();
            return Ok(false);
        };

        self.stdout_after_exit.clear();
        self.write_results.clear();
        let options = OutputOptions {
            mode: self.output_mode,
            root: self.root.clone(),
            out_dir: Some(self.root.join("cssforge-out")),
            suffix: ".modern.css".to_string(),
        };

        let mut changed = 0usize;
        for file in &report.files {
            let original = fs::read_to_string(&file.path)
                .with_context(|| format!("failed to read {} before apply", file.path.display()))?;
            let transformed = apply_selected_plans(&original, &file.plans, true)?;
            if transformed == original {
                continue;
            }
            changed += 1;
            let result = write_result(&file.path, &original, &transformed, &options)?;
            if let Some(stdout) = &result.stdout {
                self.stdout_after_exit
                    .push(format!("/* {} */\n{}", file.path.display(), stdout));
            }
            self.write_results.push(result);
        }

        self.status = match self.output_mode {
            OutputMode::DryRun => {
                format!("Dry run complete: {changed} file(s) would be modified; nothing written.")
            }
            OutputMode::Stdout => {
                self.should_quit = true;
                format!("Prepared {changed} modernized file(s) for stdout; exiting TUI.")
            }
            _ => format!("Modernization complete: {changed} file(s) successfully transformed!"),
        };
        Ok(true)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                if self.screen == Screen::Diff {
                    self.screen = Screen::Output;
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Enter => {
                self.handle_enter()?;
            }
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Left | KeyCode::Char('b') => {
                self.handle_back();
            }
            KeyCode::Tab => self.next_step(),
            KeyCode::BackTab => self.previous_step(),
            KeyCode::Char('1') => self.screen = Screen::Files,
            KeyCode::Char('2') => self.screen = Screen::Rules,
            KeyCode::Char('3') => {
                let _ = self.analyze();
                self.screen = Screen::Output;
            }
            KeyCode::Char('4') => {
                if !self.write_results.is_empty() {
                    self.screen = Screen::Done;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Right | KeyCode::Char('l') => {
                if self.screen != Screen::Done {
                    self.handle_enter()?;
                }
            }
            KeyCode::PageUp => self.page_up(),
            KeyCode::PageDown => self.page_down(),
            KeyCode::Char(' ') => self.toggle_current()?,
            KeyCode::Char('a') => self.handle_a_key()?,
            KeyCode::Char('d') | KeyCode::Char('v') => {
                if self.screen == Screen::Output {
                    self.refresh_diff()?;
                    self.screen = Screen::Diff;
                } else if self.screen == Screen::Diff {
                    self.screen = Screen::Output;
                }
            }
            KeyCode::Char('p') => self.cycle_preset(),
            KeyCode::Char('r') => {
                if self.screen == Screen::Done {
                    self.refresh_files()?;
                    self.screen = Screen::Files;
                    self.status = "Started new refactoring session.".to_string();
                } else {
                    self.refresh_files()?;
                    self.status = "Refreshed CSS files from disk.".to_string();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_enter(&mut self) -> Result<()> {
        match self.screen {
            Screen::Files => {
                if self.selected_files().is_empty() {
                    self.status =
                        "Please select at least one file before proceeding (Space to select)."
                            .to_string();
                } else {
                    self.screen = Screen::Rules;
                    self.status =
                        "Select rules or preset, then press [Enter] to choose output mode."
                            .to_string();
                }
            }
            Screen::Rules => {
                if self.enabled_rules().is_empty() {
                    self.status = "Please enable at least one rule before proceeding (Space to toggle or p for preset).".to_string();
                } else {
                    self.analyze()?;
                    self.screen = Screen::Output;
                    self.status =
                        "Select output setting and press [Enter] to apply transformations."
                            .to_string();
                }
            }
            Screen::Output => {
                let applied = self.apply()?;
                if applied && (self.output_mode != OutputMode::Stdout || !self.should_quit) {
                    self.screen = Screen::Done;
                }
            }
            Screen::Done => {
                self.should_quit = true;
            }
            Screen::Diff => {
                self.screen = Screen::Output;
            }
        }
        Ok(())
    }

    fn handle_back(&mut self) {
        match self.screen {
            Screen::Files => {}
            Screen::Rules => {
                self.screen = Screen::Files;
                self.status = "Select CSS files to modernize, then press [Enter].".to_string();
            }
            Screen::Output => {
                self.screen = Screen::Rules;
                self.status = "Select rules or preset, then press [Enter].".to_string();
            }
            Screen::Done => {
                self.screen = Screen::Output;
                self.status = "Choose output setting and press [Enter] to apply.".to_string();
            }
            Screen::Diff => {
                self.screen = Screen::Output;
            }
        }
    }

    fn handle_a_key(&mut self) -> Result<()> {
        match self.screen {
            Screen::Files => {
                let all_selected = self.files.iter().all(|f| f.selected);
                for file in &mut self.files {
                    file.selected = !all_selected;
                }
                self.status = if all_selected {
                    "Deselected all files.".to_string()
                } else {
                    "Selected all files.".to_string()
                };
            }
            Screen::Rules => {
                let all_enabled = self.rules.iter().all(|r| r.enabled);
                for rule in &mut self.rules {
                    rule.enabled = !all_enabled;
                }
                self.preset = Preset::Custom;
                self.status = if all_enabled {
                    "Disabled all rules.".to_string()
                } else {
                    "Enabled all rules.".to_string()
                };
            }
            Screen::Output => {
                self.analyze()?;
                self.status = "Refreshed workspace analysis.".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    fn toggle_current(&mut self) -> Result<()> {
        match self.screen {
            Screen::Files => {
                if let Some(item) = self.files.get_mut(self.file_cursor) {
                    item.selected = !item.selected;
                    let file_name = item.path.file_name().unwrap_or_default().to_string_lossy();
                    self.status = if item.selected {
                        format!("Selected {file_name}")
                    } else {
                        format!("Deselected {file_name}")
                    };
                }
            }
            Screen::Rules => {
                if let Some(item) = self.rules.get_mut(self.rule_cursor) {
                    item.enabled = !item.enabled;
                    self.preset = Preset::Custom;
                    let title = item.definition.title;
                    self.status = if item.enabled {
                        format!("Enabled rule: {title}")
                    } else {
                        format!("Disabled rule: {title}")
                    };
                }
            }
            Screen::Output => {
                self.output_mode = OutputMode::ALL[self.output_cursor];
                self.status = format!("Selected output mode: {}", self.output_mode.label());
            }
            Screen::Diff => self.toggle_current_plan()?,
            _ => {}
        }
        Ok(())
    }

    fn move_up(&mut self) {
        match self.screen {
            Screen::Files => self.file_cursor = self.file_cursor.saturating_sub(1),
            Screen::Rules => self.rule_cursor = self.rule_cursor.saturating_sub(1),
            Screen::Output => {
                self.output_cursor = self.output_cursor.saturating_sub(1);
                self.output_mode = OutputMode::ALL[self.output_cursor];
            }
            Screen::Diff => {
                self.plan_cursor = self.plan_cursor.saturating_sub(1);
                self.diff_scroll = 0;
                let _ = self.refresh_diff();
            }
            _ => {}
        }
    }

    fn move_down(&mut self) {
        match self.screen {
            Screen::Files => {
                self.file_cursor = (self.file_cursor + 1).min(self.files.len().saturating_sub(1));
            }
            Screen::Rules => {
                self.rule_cursor = (self.rule_cursor + 1).min(self.rules.len().saturating_sub(1));
            }
            Screen::Output => {
                self.output_cursor =
                    (self.output_cursor + 1).min(OutputMode::ALL.len().saturating_sub(1));
                self.output_mode = OutputMode::ALL[self.output_cursor];
            }
            Screen::Diff => {
                self.plan_cursor =
                    (self.plan_cursor + 1).min(self.plan_positions().len().saturating_sub(1));
                self.diff_scroll = 0;
                let _ = self.refresh_diff();
            }
            _ => {}
        }
    }

    fn page_up(&mut self) {
        match self.screen {
            Screen::Diff => {
                self.diff_scroll = self.diff_scroll.saturating_sub(10);
            }
            Screen::Files => {
                self.file_cursor = self.file_cursor.saturating_sub(10);
            }
            Screen::Rules => {
                self.rule_cursor = self.rule_cursor.saturating_sub(10);
            }
            _ => {}
        }
    }

    fn page_down(&mut self) {
        match self.screen {
            Screen::Diff => {
                self.diff_scroll = self.diff_scroll.saturating_add(10);
            }
            Screen::Files => {
                self.file_cursor = (self.file_cursor + 10).min(self.files.len().saturating_sub(1));
            }
            Screen::Rules => {
                self.rule_cursor = (self.rule_cursor + 10).min(self.rules.len().saturating_sub(1));
            }
            _ => {}
        }
    }

    fn next_step(&mut self) {
        let idx = Screen::STEPS
            .iter()
            .position(|s| *s == self.screen)
            .unwrap_or(0);
        let next = Screen::STEPS[(idx + 1) % Screen::STEPS.len()];
        if next == Screen::Output {
            let _ = self.analyze();
        }
        self.screen = next;
    }

    fn previous_step(&mut self) {
        let idx = Screen::STEPS
            .iter()
            .position(|s| *s == self.screen)
            .unwrap_or(0);
        let prev = Screen::STEPS[(idx + Screen::STEPS.len() - 1) % Screen::STEPS.len()];
        self.screen = prev;
    }

    pub fn cycle_preset(&mut self) {
        let idx = Preset::ALL
            .iter()
            .position(|p| *p == self.preset)
            .unwrap_or(0);
        let next = Preset::ALL[(idx + 1) % Preset::ALL.len()];
        self.apply_preset(next);
    }

    pub fn apply_preset(&mut self, preset: Preset) {
        self.preset = preset;
        if preset != Preset::Custom {
            let enabled = preset.enabled_rules();
            for item in &mut self.rules {
                item.enabled = enabled.contains(&item.definition.id);
            }
        }
        self.status = format!("Preset set to {preset}.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use cssforge_core::{OutputMode, Preset};

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_wizard_step_flow() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("cssforge_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir)?;
        let test_css = temp_dir.join("test.css");
        fs::write(
            &test_css,
            ".btn { color: red; } .btn:hover { color: blue; }",
        )?;

        let mut app = App::new(temp_dir.clone())?;
        assert_eq!(app.screen, Screen::Files);
        assert!(!app.files.is_empty());

        // Step 1: Files -> Press Enter -> Rules
        app.handle_key(make_key(KeyCode::Enter))?;
        assert_eq!(app.screen, Screen::Rules);

        // Step 2: Rules -> Press Enter -> Output
        app.handle_key(make_key(KeyCode::Enter))?;
        assert_eq!(app.screen, Screen::Output);
        assert!(app.report.is_some());

        // Output mode selection
        app.handle_key(make_key(KeyCode::Down))?;
        assert_eq!(app.output_mode, OutputMode::OutDir);

        // Step 3: Output -> Press Enter -> Apply and Done
        app.handle_key(make_key(KeyCode::Enter))?;
        assert_eq!(app.screen, Screen::Done);
        assert!(!app.write_results.is_empty());

        // Step 4: Done -> Press Enter -> Quits
        app.handle_key(make_key(KeyCode::Enter))?;
        assert!(app.should_quit);

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }

    #[test]
    fn test_navigation_back() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("cssforge_nav_{}", std::process::id()));
        fs::create_dir_all(&temp_dir)?;
        fs::write(temp_dir.join("a.css"), "body { margin: 0; }")?;

        let mut app = App::new(temp_dir.clone())?;
        assert_eq!(app.screen, Screen::Files);

        // Advance to Rules
        app.handle_key(make_key(KeyCode::Enter))?;
        assert_eq!(app.screen, Screen::Rules);

        // Back to Files via Esc
        app.handle_key(make_key(KeyCode::Esc))?;
        assert_eq!(app.screen, Screen::Files);

        // Advance to Rules then Output
        app.handle_key(make_key(KeyCode::Enter))?;
        app.handle_key(make_key(KeyCode::Enter))?;
        assert_eq!(app.screen, Screen::Output);

        // Back to Rules via Backspace
        app.handle_key(make_key(KeyCode::Backspace))?;
        assert_eq!(app.screen, Screen::Rules);

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }

    #[test]
    fn test_rules_select_all_and_page_scroll() -> Result<()> {
        let temp_dir = std::env::temp_dir().join(format!("cssforge_all_{}", std::process::id()));
        fs::create_dir_all(&temp_dir)?;
        fs::write(temp_dir.join("a.css"), "body { margin: 0; }")?;

        let mut app = App::new(temp_dir.clone())?;
        app.handle_key(make_key(KeyCode::Enter))?;
        assert_eq!(app.screen, Screen::Rules);
        assert!(!app.rules.is_empty());

        app.handle_key(make_key(KeyCode::Char('a')))?;
        let enabled = app.rules.iter().filter(|r| r.enabled).count();
        assert!(enabled == 0 || enabled == app.rules.len());
        if enabled == 0 {
            app.handle_key(make_key(KeyCode::Char('a')))?;
        }
        assert!(app.rules.iter().all(|r| r.enabled));
        assert_eq!(app.preset, Preset::Custom);

        app.handle_key(make_key(KeyCode::Char('a')))?;
        assert!(app.rules.iter().all(|r| !r.enabled));

        app.handle_key(make_key(KeyCode::Char('a')))?;
        assert!(app.rules.iter().all(|r| r.enabled));

        app.rule_cursor = 0;
        app.handle_key(make_key(KeyCode::PageDown))?;
        assert_eq!(app.rule_cursor, 10.min(app.rules.len() - 1));
        app.handle_key(make_key(KeyCode::PageUp))?;
        assert_eq!(app.rule_cursor, 0);

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }
}
