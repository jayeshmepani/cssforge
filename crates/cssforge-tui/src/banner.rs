use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// ANSI Shadow wordmark (~79 cols) for large viewports with clean letter spacing.
const BANNER_WIDE: &[&str] = &[
    r"  ██████╗  ███████╗  ███████╗   ███████╗   ██████╗   ██████╗   ██████╗  ███████╗",
    r" ██╔════╝  ██╔════╝  ██╔════╝   ██╔════╝  ██╔═══██╗ ██╔═══██╗ ██╔════╝  ██╔════╝",
    r" ██║       ███████╗  ███████╗   █████╗    ██║   ██║ ██████╔╝  ██║  ███╗ █████╗  ",
    r" ██║       ╚════██║  ╚════██║   ██╔══╝    ██║   ██║ ██╔══██╗  ██║   ██║ ██╔══╝  ",
    r" ╚██████╗  ███████║  ███████║   ██║       ╚██████╔╝ ██║  ╚██╗ ╚██████╔╝ ███████╗",
    r"  ╚═════╝  ╚══════╝  ╚══════╝   ╚═╝        ╚═════╝   ╚═╝  ╚═╝  ╚═════╝  ╚══════╝",
];

/// Slant wordmark (~50 cols) for medium viewports.
const BANNER_COMPACT: &[&str] = &[
    r"   ________________ __________  ____  ____________",
    r"  / ____/ ___/ ___// ____/ __ \/ __ \/ ____/ ____/",
    r" / /    \__ \\__ \/ /_  / / / / /_/ / / __/ __/   ",
    r"/ /___ ___/ /__/ / __/ / /_/ / _, _/ /_/ / /___   ",
    r"\____//____/____/_/    \____/_/ |_|\____/_____/   ",
];

/// Small Slant wordmark (~39 cols) for narrow viewports.
const BANNER_MINI: &[&str] = &[
    r"  _____________________  ___  _________",
    r" / ___/ __/ __/ __/ __ \/ _ \/ ___/ __/",
    r"/ /___\ \_\ \/ _// /_/ / , _/ (_ / _/  ",
    r"\___/___/___/_/  \____/_/|_|\___/___/  ",
];

const TAGLINE: &str = "  Forward semantic CSS modernization  ·  nest  ·  refactor  ·  review";

pub fn art_for_width(width: u16) -> Option<&'static [&'static str]> {
    if width >= 82 {
        Some(BANNER_WIDE)
    } else if width >= 54 {
        Some(BANNER_COMPACT)
    } else if width >= 42 {
        Some(BANNER_MINI)
    } else {
        None
    }
}

/// Rows used by the banner block (art + tagline + trailing blank), or 0 if it should hide.
pub fn reserved_height(width: u16, available: u16) -> u16 {
    let art_rows = art_for_width(width)
        .map(|art| art.len() as u16)
        .unwrap_or(1);
    let needed = art_rows + 2;
    if available < needed + 6 { 0 } else { needed }
}

pub fn render_lines(width: u16) -> Vec<Line<'static>> {
    let mut lines = match art_for_width(width) {
        Some(art) => paint_art(art),
        None => vec![Line::from(vec![
            Span::styled(
                " CSSFORGE ",
                Style::default()
                    .fg(Color::Rgb(6, 36, 28))
                    .bg(Color::Rgb(16, 185, 129))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  forward CSS modernization",
                Style::default().fg(Color::Rgb(110, 155, 135)),
            ),
        ])],
    };
    lines.push(Line::styled(
        TAGLINE,
        Style::default().fg(Color::Rgb(110, 155, 135)),
    ));
    lines.push(Line::raw(""));
    lines
}

fn paint_art(art: &[&str]) -> Vec<Line<'static>> {
    let max_w = art.iter().map(|row| row.chars().count()).max().unwrap_or(1);
    art.iter()
        .map(|row| {
            let spans = row
                .chars()
                .enumerate()
                .map(|(col, ch)| {
                    if ch == ' ' {
                        Span::raw(" ")
                    } else {
                        Span::styled(
                            ch.to_string(),
                            Style::default().fg(gradient_color(col, max_w)),
                        )
                    }
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect()
}

fn gradient_color(col: usize, width: usize) -> Color {
    let t = if width <= 1 {
        0.0
    } else {
        col as f32 / (width - 1) as f32
    };
    // Deep jade → jewel emerald → frosted mint.
    let stops = [
        (10u8, 122u8, 98u8),
        (16u8, 185u8, 129u8),
        (154u8, 230u8, 180u8),
    ];
    let (idx, local) = if t < 0.5 {
        (0, t / 0.5)
    } else {
        (1, (t - 0.5) / 0.5)
    };
    let (r1, g1, b1) = stops[idx];
    let (r2, g2, b2) = stops[idx + 1];
    Color::Rgb(
        lerp(r1, r2, local),
        lerp(g1, g2, local),
        lerp(b1, b2, local),
    )
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    let t = t.clamp(0.0, 1.0);
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_art_by_width() {
        assert_eq!(art_for_width(120).unwrap().len(), BANNER_WIDE.len());
        assert_eq!(art_for_width(90).unwrap().len(), BANNER_WIDE.len());
        assert_eq!(art_for_width(60).unwrap().len(), BANNER_COMPACT.len());
        assert_eq!(art_for_width(45).unwrap().len(), BANNER_MINI.len());
        assert!(art_for_width(35).is_none());
    }

    #[test]
    fn hides_when_viewport_is_short() {
        assert_eq!(reserved_height(120, 8), 0);
        assert!(reserved_height(120, 24) > 0);
    }
}
