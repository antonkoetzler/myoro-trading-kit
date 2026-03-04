//! Data-source configuration modal dialog.
use crate::tui::router::DataConfigDialog;
use crate::tui::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout as RLayout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, dialog: &DataConfigDialog) {
    let area = centered_rect(70, 80, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::tab_selected())
        .title(Span::styled(
            format!(" Configure: {} ", dialog.source_name),
            Theme::tab_selected().add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(Theme::BG()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if dialog.fields.is_empty() {
        let para = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No configuration required for this source.",
                Theme::body(),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Press [Enter] or [s] to continue.",
                Theme::dim(),
            )),
        ]);
        f.render_widget(para, inner);
        return;
    }

    // Split: fields list (left 60%) | value editor / hint (right 40%)
    let horiz = RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(inner);

    // Field list
    let items: Vec<ListItem> = dialog
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let sel = i == dialog.field_sel;
            let style = if sel {
                Theme::tab_selected().add_modifier(Modifier::BOLD)
            } else {
                Theme::dim()
            };
            let cursor = if sel { "▸" } else { " " };
            let val_display = if f.is_dropdown() {
                format!("  {} [←/→]", f.value)
            } else {
                format!("  {}", f.value)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} {:<28}", cursor, f.label), style),
                Span::styled(val_display, if sel { Theme::body() } else { Theme::dim() }),
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(dialog.field_sel));

    let fields_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::block_border())
        .title(Span::styled(" Fields ", Theme::block_title()));

    let list = List::new(items).block(fields_block);
    f.render_stateful_widget(list, horiz[0], &mut list_state);

    // Right: current field editor / dropdown options
    render_field_editor(f, horiz[1], dialog);
}

fn render_field_editor(f: &mut Frame, area: Rect, dialog: &DataConfigDialog) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::block_border())
        .title(Span::styled(" Edit ", Theme::block_title()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(field) = dialog.fields.get(dialog.field_sel) else {
        return;
    };

    if field.is_dropdown() {
        // Show dropdown options list with current highlighted
        let lines: Vec<Line> = field
            .options
            .iter()
            .map(|opt| {
                let is_cur = *opt == field.value;
                let prefix = if is_cur { "● " } else { "  " };
                let style = if is_cur {
                    Theme::tab_selected().add_modifier(Modifier::BOLD)
                } else {
                    Theme::dim()
                };
                Line::from(Span::styled(format!("{}{}", prefix, opt), style))
            })
            .collect();
        let mut hint = lines;
        hint.push(Line::from(""));
        hint.push(Line::from(Span::styled("  ←/→ to change", Theme::dim())));
        f.render_widget(Paragraph::new(hint), inner);
    } else {
        // Text field: show current value or editing buffer
        let display = if dialog.editing {
            format!("{}\u{2588}", dialog.input) // cursor block
        } else {
            field.value.to_string()
        };
        let edit_hint = if dialog.editing {
            "  Enter = confirm  Esc = cancel"
        } else {
            "  Enter = edit"
        };
        let lines = vec![
            Line::from(Span::styled(field.label.as_str(), Theme::metrics_label())),
            Line::from(""),
            Line::from(Span::styled(
                display,
                if dialog.editing {
                    Theme::body().add_modifier(Modifier::BOLD)
                } else {
                    Theme::dim()
                },
            )),
            Line::from(""),
            Line::from(Span::styled(edit_hint, Theme::dim())),
        ];
        f.render_widget(Paragraph::new(lines), inner);
    }

    // Bottom shortcuts hint outside the edit area
    let shortcuts_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(2),
        width: area.width,
        height: 1,
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  ↑/↓ field  [s] save & continue  Esc close",
            Theme::dim(),
        ))),
        shortcuts_area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vert = RLayout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    RLayout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vert[1])[1]
}
