// NOTE: Exceeds 300-line limit — eight distinct modal dialog renderers; splitting would create excessive cross-module imports for one-off UI functions. See docs/ai-rules/file-size.md
//! Modal dialog renderers: theme overlay, bankroll prompt, currency picker,
//! live-mode confirm, discover filter dialog, copy-add dialog.

use crate::tui::layout::{ShortcutPair, TABS};
use crate::tui::theme::{Theme, ThemePalette, COLOR_PRESETS, THEME_CREATOR_ROLES};
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

fn bordered_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", title))
        .border_style(Theme::block_border())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::BG()))
}

pub fn render_shortcuts_screen(f: &mut Frame, shortcuts: &[(String, Vec<ShortcutPair>)]) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(Theme::BG())),
        area,
    );
    if area.width == 0 || area.height == 0 {
        return;
    }
    // Calculate height per section by simulating line-breaks at the actual terminal width.
    let content_w = area.width.saturating_sub(4) as usize; // subtract block borders
    let mut constraints: Vec<Constraint> = shortcuts
        .iter()
        .map(|(_, pairs)| {
            let mut line_count = 1usize;
            let mut line_len = 0usize;
            for (k, a) in pairs {
                let group_len = 2 + k.len() + 2 + a.len() + 2;
                if content_w > 0 && line_len + group_len > content_w && line_len > 0 {
                    line_count += 1;
                    line_len = 0;
                }
                line_len += group_len;
            }
            Constraint::Length((line_count + 2) as u16) // +2 for top/bottom block borders
        })
        .collect();
    constraints.push(Constraint::Min(0));
    let chunks = RLayout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .margin(0)
        .split(area);
    for (i, (category, pairs)) in shortcuts.iter().enumerate() {
        if i >= chunks.len().saturating_sub(1) {
            break;
        }
        let block = bordered_block(category);
        let rect = chunks[i];
        let block_inner = block.inner(rect);
        f.render_widget(block, rect);
        let w = block_inner.width as usize;
        let mut lines: Vec<Line> = Vec::new();
        let mut line_spans: Vec<Span> = Vec::new();
        let mut len = 0usize;
        for (k, a) in pairs {
            let group_len = 2 + k.len() + 2 + a.len() + 2;
            if len + group_len > w && !line_spans.is_empty() {
                lines.push(Line::from(std::mem::take(&mut line_spans)));
                len = 0;
            }
            line_spans.push(Span::styled(format!("[{}] ", k), Theme::key()));
            line_spans.push(Span::styled(format!("{}  ", a), Theme::body()));
            len += group_len;
        }
        if !line_spans.is_empty() {
            lines.push(Line::from(line_spans));
        }
        let para = Paragraph::new(if lines.is_empty() {
            vec![Line::from("")]
        } else {
            lines
        });
        f.render_widget(para, block_inner);
    }
    if let Some(&r) = chunks.get(shortcuts.len()) {
        if r.height > 0 {
            f.render_widget(Block::default().style(Style::default().bg(Theme::BG())), r);
        }
    }
}

pub fn render_bankroll_prompt(f: &mut Frame, input: &str) {
    let area = f.area();
    let w = 50u16.min(area.width);
    let rect = centered_rect(w, 4, area);
    let block = bordered_block(" Set Paper Bankroll ");
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let para = Paragraph::new(vec![
        Line::from(vec![
            Span::raw("Amount: "),
            Span::raw(input),
            Span::styled("▌", Theme::body()),
        ]),
        Line::from(vec![
            Span::styled("[Enter] ", Theme::key()),
            Span::raw("confirm  "),
            Span::styled("[Esc] ", Theme::key()),
            Span::raw("cancel"),
        ]),
    ])
    .style(Theme::body());
    f.render_widget(para, inner);
}

pub fn render_currency_picker(f: &mut Frame, currencies: &[&str], selected: usize, filter: &str) {
    let area = f.area();
    let w = 36u16.min(area.width);
    let h = ((currencies.len() + 4) as u16).min(area.height).max(6);
    let rect = centered_rect(w, h, area);
    let block = bordered_block(" P&L Currency ");
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Filter: ", Theme::block_title()),
            Span::raw(filter),
            Span::styled("▌", Theme::body()),
        ]),
        Line::from(""),
    ];
    for (i, c) in currencies.iter().enumerate() {
        let mark = if i == selected { "► " } else { "  " };
        lines.push(Line::from(vec![
            Span::raw(mark),
            Span::styled(*c, Theme::body()),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[Enter] ", Theme::key()),
        Span::raw("select  "),
        Span::styled("[Esc] ", Theme::key()),
        Span::raw("cancel"),
    ]));
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

pub fn render_live_confirm(f: &mut Frame, tab_index: usize) {
    let area = f.area();
    let tab_name = TABS.get(tab_index).copied().unwrap_or("?");
    let msg = format!("Switch {} to Live trading? (y/n)", tab_name);
    let bw = (msg.len() as u16 + 4).max(24).min(area.width);
    let rect = centered_rect(bw, 4, area);
    let block = bordered_block(" Confirm ");
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    f.render_widget(Paragraph::new(msg.as_str()).style(Theme::body()), inner);
}

pub fn render_discover_filter_dialog(
    f: &mut Frame,
    title: &str,
    options: &[&str],
    selected: usize,
) {
    let area = f.area();
    let sel = selected.min(options.len().saturating_sub(1));
    let content_w = options.iter().map(|s| s.len()).max().unwrap_or(8).max(28) as u16;
    let rect_w = (content_w + 4).min(area.width);
    let rect_h = (options.len() as u16 + 2).min(area.height);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} — Enter apply, Esc cancel ", title))
        .border_style(Theme::block_border())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::BG()));
    let rect = centered_rect(rect_w, rect_h, area);
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let lines: Vec<Line> = options
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let style = if i == sel {
                Theme::tab_selected()
            } else {
                Theme::body()
            };
            Line::from(Span::styled(*s, style))
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(Theme::body()), inner);
}

pub fn render_copy_add_dialog(f: &mut Frame, search_query: &str, option_rows: &[(String, bool)]) {
    let area = f.area();
    let w = 56u16.min(area.width).max(40);
    let list_h = option_rows.len().min(12) as u16;
    let h = (4 + list_h).min(area.height).max(8);
    let rect = centered_rect(w, h, area);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add trader — ↑↓ select, Enter add, Esc cancel ")
        .border_style(Theme::block_border())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::BG()));
    let inner = block.inner(rect);
    f.render_widget(block, rect);
    let chunks = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(2),
            Constraint::Length(1),
        ])
        .split(inner);
    let search_line = Line::from(vec![
        Span::styled("Search or paste address: ", Theme::block_title()),
        Span::raw(search_query),
        Span::styled("_", Theme::body()),
    ]);
    f.render_widget(Paragraph::new(search_line).style(Theme::body()), chunks[0]);
    let list_lines: Vec<Line> = option_rows
        .iter()
        .map(|(text, selected)| {
            let style = if *selected {
                Theme::tab_selected()
            } else {
                Theme::body()
            };
            let mark = if *selected { "► " } else { "  " };
            Line::from(Span::styled(format!("{}{}", mark, text), style))
        })
        .collect();
    let list_para = Paragraph::new(if list_lines.is_empty() {
        vec![Line::from(Span::styled(
            "Type a username to search, or paste 0x… address",
            Theme::dim(),
        ))]
    } else {
        list_lines
    });
    f.render_widget(list_para, chunks[1]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[Enter] ", Theme::key()),
            Span::raw("add  "),
            Span::styled("[Esc] ", Theme::key()),
            Span::raw("cancel"),
        ]))
        .style(Theme::dim()),
        chunks[2],
    );
}

#[allow(clippy::too_many_arguments)]
pub fn render_theme_screen(
    f: &mut Frame,
    theme_selection: usize,
    theme_names: &[String],
    current_theme_index: usize,
    in_creator: bool,
    creator_role: usize,
    creator_color_idx: usize,
    editor_palette: Option<&ThemePalette>,
) {
    let area = f.area();
    f.render_widget(
        Block::default().style(Style::default().bg(Theme::BG())),
        area,
    );
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Theming — T / Esc close ")
        .border_style(Theme::block_border())
        .title_style(Theme::block_title())
        .style(Style::default().bg(Theme::BG()));
    let block_inner = block.inner(area);
    f.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();
    if in_creator {
        lines.push(Line::from(
            "Theme Creator — j/k role, h/l color, s save, Esc back",
        ));
        lines.push(Line::from(""));
        if let Some(p) = editor_palette {
            for (i, name) in THEME_CREATOR_ROLES.iter().enumerate() {
                let mark = if i == creator_role { "► " } else { "  " };
                let c = p.role_color(i);
                lines.push(Line::from(format!(
                    "{}{}: [{:3},{:3},{:3}]",
                    mark, name, c[0], c[1], c[2]
                )));
            }
            lines.push(Line::from(""));
            let ci = creator_color_idx.min(COLOR_PRESETS.len().saturating_sub(1));
            let cp = COLOR_PRESETS[ci];
            lines.push(Line::from(format!(
                "Color preset {}/{}: {:?}  (h/l change)",
                ci + 1,
                COLOR_PRESETS.len(),
                cp
            )));
        }
    } else {
        lines.push(Line::from(
            "↑↓/jk select  Enter apply  e Export  i Import  c Creator",
        ));
        lines.push(Line::from(""));
        for (i, name) in theme_names.iter().enumerate() {
            let mark = if i == theme_selection { "► " } else { "  " };
            let cur = if i == current_theme_index {
                " (active)"
            } else {
                ""
            };
            lines.push(Line::from(format!("{}{}{}", mark, name, cur)));
        }
        lines.push(Line::from(""));
        let n = theme_names.len();
        lines.push(Line::from(format!(
            "{}[e] Export to theme_export.toml",
            if theme_selection == n { "► " } else { "  " }
        )));
        lines.push(Line::from(format!(
            "{}[i] Import from theme_import.toml",
            if theme_selection == n + 1 {
                "► "
            } else {
                "  "
            }
        )));
        lines.push(Line::from(format!(
            "{}[c] Theme creator",
            if theme_selection == n + 2 {
                "► "
            } else {
                "  "
            }
        )));
    }
    f.render_widget(Paragraph::new(lines).style(Theme::body()), block_inner);
}

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w.min(area.width),
        height: h.min(area.height),
    }
}
