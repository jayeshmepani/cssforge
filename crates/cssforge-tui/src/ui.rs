use crate::app::{App, Screen};
use cssforge_core::{OutputMode, RuleSection, Safety, SafetyLevel};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

const ACCENT: Color = Color::Cyan;
const MUTED: Color = Color::DarkGray;

pub fn draw(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(8),
            Constraint::Length(3),
        ])
        .split(area);

    render_header(frame, chunks[0], app);
    match app.screen {
        Screen::Files => render_files(frame, chunks[1], app),
        Screen::Rules => render_rules(frame, chunks[1], app),
        Screen::Output => render_output(frame, chunks[1], app),
        Screen::Done => render_done(frame, chunks[1], app),
        Screen::Diff => render_diff(frame, chunks[1], app),
    }
    render_footer(frame, chunks[2], app);
}

fn render_header(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut step_spans = vec![
        Span::styled(
            " CSSForge ",
            Style::default()
                .fg(Color::Black)
                .bg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
    ];

    let current_step_idx = app.screen.step_index();
    let step_labels = [
        "1: Select Files",
        "2: Select Rules",
        "3: Output Settings",
        "4: Done",
    ];

    for (idx, label) in step_labels.iter().enumerate() {
        if idx > 0 {
            step_spans.push(Span::styled(" ➔ ", Style::default().fg(MUTED)));
        }
        let style = match current_step_idx {
            Some(curr) if curr == idx => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
            Some(curr) if curr > idx => Style::default().fg(Color::Green),
            _ => Style::default().fg(Color::DarkGray),
        };

        let icon = match current_step_idx {
            Some(curr) if curr == idx => "▶ ",
            Some(curr) if curr > idx => "✓ ",
            _ => "○ ",
        };

        step_spans.push(Span::styled(format!("{icon}{label}"), style));
    }

    if app.screen == Screen::Diff {
        step_spans.push(Span::styled(
            "  [Diff Inspector]",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let path = app.root.display().to_string();
    let selected_files = app.files.iter().filter(|f| f.selected).count();
    let selected_rules = app.rules.iter().filter(|r| r.enabled).count();

    let line2 = Line::from(vec![
        Span::styled("Root: ", Style::default().fg(MUTED)),
        Span::raw(path),
        Span::raw("  │  "),
        Span::styled("Files: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{selected_files}/{}", app.files.len()),
            Style::default().fg(if selected_files > 0 {
                Color::Green
            } else {
                Color::Red
            }),
        ),
        Span::raw("  │  "),
        Span::styled("Rules: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{selected_rules}/{} ({})", app.rules.len(), app.preset),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  │  "),
        Span::styled("Output: ", Style::default().fg(MUTED)),
        Span::styled(app.output_mode.label(), Style::default().fg(Color::Magenta)),
    ]);

    let header = Paragraph::new(Text::from(vec![Line::from(step_spans), line2]))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, area);
}

fn render_files(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut lines = Vec::new();
    let selected_count = app.files.iter().filter(|f| f.selected).count();

    lines.push(Line::styled(
        " Choose the CSS files you want to modernize. Press [Space] to toggle, [a] for all, [Enter] to confirm.",
        Style::default().fg(Color::Cyan),
    ));
    lines.push(Line::raw(""));

    if app.files.is_empty() {
        lines.push(Line::styled(
            " No .css files found under the selected directory.",
            Style::default().fg(Color::Yellow),
        ));
    } else {
        for (idx, item) in app.files.iter().enumerate() {
            let relative = item.path.strip_prefix(&app.root).unwrap_or(&item.path);
            let marker = if item.selected { "[x]" } else { "[ ]" };
            let style = if idx == app.file_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD)
            } else if item.selected {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(MUTED)
            };
            lines.push(Line::styled(
                format!(" {marker} {}", relative.display()),
                style,
            ));
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!(
            " Selected: {} of {} file(s)  ·  Press [ENTER] to proceed to Rules ➔",
            selected_count,
            app.files.len()
        ),
        Style::default().fg(if selected_count > 0 {
            Color::Green
        } else {
            Color::Yellow
        }),
    ));

    let offset = scroll_offset(app.file_cursor, area.height.saturating_sub(4) as usize);
    let widget = Paragraph::new(Text::from(lines))
        .block(Block::bordered().title(
            " Step 1 of 4: Select CSS Files — [Enter] Next ➔  ·  [Space] Toggle  ·  [a] All ",
        ))
        .scroll((offset as u16, 0));
    frame.render_widget(widget, area);
}

fn render_rules(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let enabled_count = app.rules.iter().filter(|r| r.enabled).count();
    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(" Active Preset: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("[{}]", app.preset),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("   (Press ", Style::default().fg(MUTED)),
        Span::styled(
            "p",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " to cycle presets: Conservative ➔ Modern ➔ Refactor ➔ Aggressive ➔ Custom)",
            Style::default().fg(MUTED),
        ),
    ]));
    lines.push(Line::styled(
        " ⚠ DISCLAIMER: CSSForge is a strictly forward semantic modernization & refactoring engine.",
        Style::default().fg(Color::Yellow),
    ));
    lines.push(Line::styled(
        "   Backward / reverse demodernization is unsupported. Always maintain Git backups before applying changes.",
        Style::default().fg(MUTED),
    ));
    lines.push(Line::raw(""));

    let mut current_section: Option<RuleSection> = None;
    let mut cursor_line_idx = 0;
    let mut line_counter = 4;

    for (idx, item) in app.rules.iter().enumerate() {
        if current_section != Some(item.definition.section) {
            current_section = Some(item.definition.section);
            if idx > 0 {
                lines.push(Line::raw(""));
                line_counter += 1;
            }
            let (header_title, header_color) = match item.definition.section {
                RuleSection::Modernize => (
                    "── MODERNIZE (Native Nesting, Range Syntax & Modern Selectors) ──────────────────────",
                    Color::Cyan,
                ),
                RuleSection::Refactor => (
                    "── REFACTOR (Consolidation, Deduplication & Structural Cleanup) ─────────────────────",
                    Color::LightBlue,
                ),
            };
            lines.push(Line::styled(
                format!(" {}", header_title),
                Style::default()
                    .fg(header_color)
                    .add_modifier(Modifier::BOLD),
            ));
            line_counter += 1;
        }

        if idx == app.rule_cursor {
            cursor_line_idx = line_counter;
        }
        line_counter += 1;

        let marker = if item.enabled { "[x]" } else { "[ ]" };
        let level = match item.definition.safety_level {
            SafetyLevel::AnalysisOnly => "L0",
            SafetyLevel::FormattingOnly => "L1",
            SafetyLevel::ProvenLocalRefactor => "L2",
            SafetyLevel::SemanticReview => "L3",
            SafetyLevel::Architectural => "L4",
        };
        let text = format!(
            "  {marker} {:<34} [{:<21} · {}]  {}",
            item.definition.title, item.definition.category, level, item.definition.description
        );
        let style = if idx == app.rule_cursor {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD)
        } else if item.enabled {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(MUTED)
        };
        lines.push(Line::styled(text, style));
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!(
            " Enabled: {} of {} rule(s)  ·  Press [ENTER] to confirm & choose Output Settings ➔",
            enabled_count,
            app.rules.len()
        ),
        Style::default().fg(if enabled_count > 0 {
            Color::Green
        } else {
            Color::Yellow
        }),
    ));

    let offset = scroll_offset(cursor_line_idx, area.height.saturating_sub(4) as usize);
    let widget = Paragraph::new(Text::from(lines))
        .block(
            Block::bordered().title(" Step 2 of 4: Select Rules — [Enter] Next ➔  ·  [Space] Toggle  ·  [p] Preset  ·  [Esc] Back ")
        )
        .scroll((offset as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn render_output(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left Pane: Output Modes
    let mut left_lines = vec![
        Line::styled(
            " Select how modernized CSS will be saved:",
            Style::default().fg(Color::Cyan),
        ),
        Line::raw(""),
    ];

    for (idx, mode) in OutputMode::ALL.iter().enumerate() {
        let is_selected = *mode == app.output_mode;
        let is_highlighted = idx == app.output_cursor;
        let marker = if is_selected { "●" } else { "○" };
        let tag = match mode {
            OutputMode::NewFile => " [Default - Safe]",
            OutputMode::OutDir => " [Separate folder]",
            OutputMode::OverwriteWithBackup => " [Safe in-place + backup]",
            OutputMode::Overwrite => " [Direct modify in place]",
            OutputMode::DryRun => " [Preview only]",
            OutputMode::Patch => " [Diff patch]",
            OutputMode::Stdout => " [Terminal print]",
        };
        let label = format!(" {} {}{}", marker, mode.label(), tag);
        let style = if is_highlighted {
            Style::default()
                .fg(Color::Black)
                .bg(Color::LightCyan)
                .add_modifier(Modifier::BOLD)
        } else if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };
        left_lines.push(Line::styled(label, style));
    }

    left_lines.push(Line::raw(""));
    left_lines.push(Line::styled(
        "Mode Description:",
        Style::default().fg(MUTED),
    ));
    let mode_desc = match app.output_mode {
        OutputMode::NewFile => {
            "Creates parallel *.modern.css files alongside originals. Zero risk to source code."
        }
        OutputMode::OutDir => {
            "Writes modernized files into <root>/cssforge-out/ maintaining relative structure."
        }
        OutputMode::OverwriteWithBackup => {
            "Backs up originals to *.bak and updates CSS files in place."
        }
        OutputMode::Overwrite => "Overwrites original CSS files directly in your worktree.",
        OutputMode::DryRun => {
            "Simulates transformations and reports statistics without writing any files."
        }
        OutputMode::Patch => "Writes unified *.patch diff files suitable for review or git apply.",
        OutputMode::Stdout => {
            "Prints all transformed CSS code directly to standard output upon exit."
        }
    };
    left_lines.push(Line::styled(
        format!(" {mode_desc}"),
        Style::default().fg(Color::White),
    ));

    let left_widget = Paragraph::new(Text::from(left_lines))
        .block(Block::bordered().title(" 1. Choose Output Destination (↑/↓ to select) "))
        .wrap(Wrap { trim: false });
    frame.render_widget(left_widget, panes[0]);

    // Right Pane: Summary & Execution
    let mut right_lines = Vec::new();
    right_lines.push(Line::styled(
        " Transformation Summary:",
        Style::default().fg(Color::Cyan),
    ));
    right_lines.push(Line::raw(""));

    if let Some(report) = &app.report {
        let s = &report.summary;
        right_lines.extend([
            Line::styled(
                format!("   Files to transform:        {:>5}", s.files),
                Style::default().fg(Color::White),
            ),
            Line::styled(
                format!("   ✓ SAFE transformations:    {:>5}", s.safe),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Line::styled(
                format!("   ⚠ REVIEW items:            {:>5}", s.review),
                Style::default().fg(Color::Yellow),
            ),
            Line::styled(
                format!(
                    "   ✗ UNSAFE / Unsupported:    {:>5}",
                    s.unsafe_count + s.unsupported
                ),
                Style::default().fg(Color::Red),
            ),
            Line::raw(""),
        ]);

        if s.safe > 0 {
            right_lines.push(Line::styled(
                format!(" Found {} safe transformation(s) ready to apply!", s.safe),
                Style::default().fg(Color::Green),
            ));
        } else {
            right_lines.push(Line::styled(
                " No safe transformations found for selected rules.",
                Style::default().fg(Color::Yellow),
            ));
        }
    } else {
        right_lines.push(Line::styled(
            " Analysis pending. Press [a] to analyze.",
            Style::default().fg(Color::Yellow),
        ));
    }

    right_lines.push(Line::raw(""));
    right_lines.push(Line::styled(
        " ┌────────────────────────────────────────────────────────┐",
        Style::default().fg(Color::Green),
    ));
    right_lines.push(Line::styled(
        " │  ➔  Press [ENTER] to Apply Transformations & Finish!   │",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD),
    ));
    right_lines.push(Line::styled(
        " └────────────────────────────────────────────────────────┘",
        Style::default().fg(Color::Green),
    ));

    right_lines.push(Line::raw(""));
    right_lines.push(Line::styled(
        " Tip: Press [d] to inspect exact code diffs and safety proofs.",
        Style::default().fg(Color::Magenta),
    ));

    let right_widget = Paragraph::new(Text::from(right_lines))
        .block(Block::bordered().title(" 2. Review & Execute "))
        .wrap(Wrap { trim: false });
    frame.render_widget(right_widget, panes[1]);
}

fn render_done(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut lines = Vec::new();
    let changed_count = app.write_results.len();

    lines.push(Line::raw(""));
    if changed_count > 0 {
        lines.push(Line::styled(
            format!(
                "  ✓ SUCCESS: Modernization Complete! Transformed {} file(s).  ",
                changed_count
            ),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));
    } else if app.output_mode == OutputMode::DryRun {
        lines.push(Line::styled(
            "  ✓ DRY RUN FINISHED: Simulation complete (no files written).  ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        lines.push(Line::styled(
            "  ✓ COMPLETE: 0 files needed transformation.  ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        format!(
            " Output Mode: {}  ·  Files Processed: {}",
            app.output_mode.label(),
            changed_count
        ),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ));
    lines.push(Line::raw(""));

    if !app.write_results.is_empty() {
        lines.push(Line::styled(
            " Transformed Files:",
            Style::default().fg(ACCENT),
        ));
        for result in &app.write_results {
            let src = compact_path(&app.root, &result.source);
            if let Some(target) = &result.written {
                let tgt = compact_path(&app.root, target);
                lines.push(Line::styled(
                    format!("   • {}  ➔  {}", src, tgt),
                    Style::default().fg(Color::Green),
                ));
            } else if let Some(backup) = &result.backup {
                let bak = compact_path(&app.root, backup);
                lines.push(Line::styled(
                    format!("   • {}  (backup: {})", src, bak),
                    Style::default().fg(Color::Green),
                ));
            } else {
                lines.push(Line::styled(
                    format!("   • {} : {}", src, result.message),
                    Style::default().fg(Color::White),
                ));
            }
        }
    }

    lines.push(Line::raw(""));
    lines.push(Line::styled(
        " ┌────────────────────────────────────────────────────────┐",
        Style::default().fg(Color::Cyan),
    ));
    lines.push(Line::styled(
        " │  [Enter] or [q] : Exit CSSForge                         │",
        Style::default().fg(Color::White),
    ));
    lines.push(Line::styled(
        " │  [r]            : Start a new refactoring session       │",
        Style::default().fg(Color::White),
    ));
    lines.push(Line::styled(
        " │  [Esc]          : Return to Output settings             │",
        Style::default().fg(Color::White),
    ));
    lines.push(Line::styled(
        " └────────────────────────────────────────────────────────┘",
        Style::default().fg(Color::Cyan),
    ));

    let widget = Paragraph::new(Text::from(lines))
        .block(
            Block::bordered().title(" Step 4 of 4: Done! — [Enter] / [q] Exit  ·  [r] Start Over "),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, area);
}

fn render_diff(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let mut lines = Vec::new();
    let positions = app.plan_positions();
    if positions.is_empty() {
        lines.push(Line::styled(
            "No transformation candidates found.",
            Style::default().fg(Color::Yellow),
        ));
    } else if let Some(report) = &app.report {
        for (idx, (fi, pi)) in positions.iter().copied().enumerate() {
            let plan = &report.files[fi].plans[pi];
            let marker = if plan.selected { "[x]" } else { "[ ]" };
            let rule_names = plan
                .rules
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let text = format!(
                " {marker} {:<8} {}  {}",
                plan.safety.label(),
                compact_path(&app.root, &plan.file),
                rule_names
            );
            let style = if idx == app.plan_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                safety_style(plan.safety)
            };
            lines.push(Line::styled(text, style));
        }
    }
    let offset = scroll_offset(app.plan_cursor, panes[0].height.saturating_sub(2) as usize);
    let list = Paragraph::new(Text::from(lines))
        .block(Block::bordered().title(" Transformation Plans (Space to toggle) "))
        .scroll((offset as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(list, panes[0]);

    let diff_lines: Vec<Line<'_>> = app
        .diff_text
        .lines()
        .map(|line| {
            let style = if line.starts_with("+++") || line.starts_with("---") {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if line.starts_with('+') {
                Style::default().fg(Color::Green)
            } else if line.starts_with('-') {
                Style::default().fg(Color::Red)
            } else if line.starts_with("@@") {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default().fg(Color::Gray)
            };
            Line::styled(line.to_string(), style)
        })
        .collect();

    let title = if let Some(plan) = app.current_plan() {
        format!(
            " Diff {} · {} · Press [Enter] or [Esc] to return ",
            plan.id, plan.safety
        )
    } else {
        " Diff (Press [Enter] or [Esc] to return) ".to_string()
    };
    let widget = Paragraph::new(Text::from(diff_lines))
        .block(Block::bordered().title(title))
        .scroll((app.diff_scroll, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(widget, panes[1]);
}

fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let key_hints: Vec<Span<'_>> = match app.screen {
        Screen::Files => vec![
            Span::styled("[Enter]", key_style()),
            Span::raw(" Next: Rules ➔   "),
            Span::styled("[Space]", key_style()),
            Span::raw(" Toggle   "),
            Span::styled("[a]", key_style()),
            Span::raw(" Select All   "),
            Span::styled("[↑↓/jk]", key_style()),
            Span::raw(" Move   "),
            Span::styled("[q]", key_style()),
            Span::raw(" Quit"),
        ],
        Screen::Rules => vec![
            Span::styled("[Enter]", key_style()),
            Span::raw(" Next: Output ➔   "),
            Span::styled("[Space]", key_style()),
            Span::raw(" Toggle   "),
            Span::styled("[p]", key_style()),
            Span::raw(" Preset   "),
            Span::styled("[Esc/b]", key_style()),
            Span::raw(" Back   "),
            Span::styled("[↑↓/jk]", key_style()),
            Span::raw(" Move   "),
            Span::styled("[q]", key_style()),
            Span::raw(" Quit"),
        ],
        Screen::Output => vec![
            Span::styled("[Enter]", key_style()),
            Span::raw(" Apply & Finish ➔   "),
            Span::styled("[↑↓/jk]", key_style()),
            Span::raw(" Select Mode   "),
            Span::styled("[d]", key_style()),
            Span::raw(" View Diff   "),
            Span::styled("[Esc/b]", key_style()),
            Span::raw(" Back   "),
            Span::styled("[q]", key_style()),
            Span::raw(" Quit"),
        ],
        Screen::Done => vec![
            Span::styled("[Enter]/[q]", key_style()),
            Span::raw(" Exit CSSForge   "),
            Span::styled("[r]", key_style()),
            Span::raw(" Start Over   "),
            Span::styled("[Esc/b]", key_style()),
            Span::raw(" Back to Settings"),
        ],
        Screen::Diff => vec![
            Span::styled("[Enter]/[Esc]", key_style()),
            Span::raw(" Return to Output   "),
            Span::styled("[Space]", key_style()),
            Span::raw(" Toggle Plan   "),
            Span::styled("[↑↓/jk]", key_style()),
            Span::raw(" Move   "),
            Span::styled("[PgUp/Dn]", key_style()),
            Span::raw(" Scroll"),
        ],
    };

    let line2 = Line::styled(
        if app.status.is_empty() {
            "Ready."
        } else {
            app.status.as_str()
        },
        Style::default().fg(Color::Cyan),
    );
    frame.render_widget(
        Paragraph::new(Text::from(vec![Line::from(key_hints), line2]))
            .block(Block::default().borders(Borders::TOP)),
        area,
    );
}

fn key_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

fn safety_style(safety: Safety) -> Style {
    match safety {
        Safety::Safe => Style::default().fg(Color::Green),
        Safety::Review => Style::default().fg(Color::Yellow),
        Safety::Unsafe => Style::default().fg(Color::Red),
        Safety::Unsupported => Style::default().fg(Color::LightRed),
        Safety::NoOp => Style::default().fg(MUTED),
    }
}

fn scroll_offset(cursor: usize, visible_rows: usize) -> usize {
    if visible_rows == 0 || cursor < visible_rows {
        0
    } else {
        cursor + 1 - visible_rows
    }
}

fn compact_path(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}
