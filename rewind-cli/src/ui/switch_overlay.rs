use crate::app::{App, StepStatus};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(50, 40, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Switch Version ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = inner.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // "Switching to ..." label
            Constraint::Length(1), // spacer
            Constraint::Min(0),   // steps
            Constraint::Length(1), // help line
        ])
        .split(content);

    // Header
    let header = Paragraph::new(format!(
        "Switching to {}...",
        app.switch_overlay_state.target_manifest
    ))
    .style(theme::text());
    f.render_widget(header, layout[0]);

    // Steps
    let step_items: Vec<ListItem> = app
        .switch_overlay_state
        .steps
        .iter()
        .map(|(kind, status)| {
            // Special handling for LockAcf when switching to latest (shown as skipped)
            let is_lock_skipped = matches!(kind, crate::app::StepKind::LockAcf)
                && matches!(status, StepStatus::Done)
                && app
                    .selected_game_entry()
                    .map(|e| e.active_manifest_id == e.latest_manifest_id)
                    .unwrap_or(false);

            if is_lock_skipped {
                let label = format!(" [\u{2014}] {} (skipped \u{2014} updates enabled)", kind.label());
                return ListItem::new(label).style(theme::text_secondary());
            }

            let (icon, style) = match status {
                StepStatus::Pending => ("[ ]", theme::text_secondary()),
                StepStatus::InProgress => ("[\u{2026}]", theme::status_warning()),
                StepStatus::Done => ("[\u{2713}]", theme::status_success()),
                StepStatus::Failed(_) => ("[\u{2717}]", theme::status_error()),
            };
            ListItem::new(format!(" {} {}", icon, kind.label())).style(style)
        })
        .collect();
    let step_list = List::new(step_items);
    f.render_widget(step_list, layout[2]);

    // Help line
    let help_text = if app.switch_overlay_state.done {
        " Done! [Esc] close "
    } else {
        " Switching... "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[3]);
}
