use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(50, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Select Version ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner.inner(Margin { horizontal: 1, vertical: 0 }));

    let cached = app
        .selected_game_entry()
        .map(|e| e.cached_manifest_ids.as_slice())
        .unwrap_or(&[]);

    if cached.is_empty() {
        let msg = Paragraph::new("No cached versions found.\nUse [D] to downgrade first.")
            .alignment(Alignment::Center)
            .style(theme::text_secondary());
        f.render_widget(msg, layout[0]);
    } else {
        let active = app
            .selected_game_entry()
            .map(|e| e.active_manifest_id.as_str())
            .unwrap_or("");

        let items: Vec<ListItem> = cached
            .iter()
            .enumerate()
            .map(|(i, manifest_id)| {
                let is_active = manifest_id == active;
                let label = if is_active {
                    format!("● {} (current)", manifest_id)
                } else {
                    format!("  {}", manifest_id)
                };

                let style = if i == app.version_picker_state.selected_index {
                    theme::selected()
                } else if is_active {
                    theme::status_success()
                } else {
                    theme::text()
                };

                ListItem::new(label).style(style)
            })
            .collect();

        let list = List::new(items);
        f.render_widget(list, layout[0]);
    }

    let help = Paragraph::new(" [↑↓] select   [Enter] switch   [Esc] cancel ")
        .style(theme::help_bar());
    f.render_widget(help, layout[1]);
}
