use crate::app::{App, VersionPickerMode};
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

    let has_info = app.version_picker_state.steam_warning
        || app.version_picker_state.error.is_some();
    let info_height: u16 = if has_info { 1 } else { 0 };

    let editing = matches!(
        app.version_picker_state.mode,
        VersionPickerMode::EditingLabel { .. }
    );
    let editor_height: u16 = if editing { 1 } else { 0 };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(info_height), // warning / error line
            Constraint::Min(0),              // version list
            Constraint::Length(editor_height), // inline label editor
            Constraint::Length(1),           // help bar
        ])
        .split(inner.inner(Margin { horizontal: 1, vertical: 0 }));

    // Warning / error line
    if app.version_picker_state.steam_warning {
        let warn = Paragraph::new(" \u{26a0} Steam is running. Quit Steam before switching.")
            .style(theme::status_warning());
        f.render_widget(warn, layout[0]);
    } else if let Some(err) = &app.version_picker_state.error {
        let err_para = Paragraph::new(format!(" {}", err))
            .style(theme::status_error());
        f.render_widget(err_para, layout[0]);
    }

    let cached = app
        .selected_game_entry()
        .map(|e| e.cached_manifest_ids.as_slice())
        .unwrap_or(&[]);

    if cached.is_empty() {
        let msg = Paragraph::new("No cached versions found.\nUse [D] to downgrade first.")
            .alignment(Alignment::Center)
            .style(theme::text_secondary());
        f.render_widget(msg, layout[1]);
    } else {
        let active = app
            .selected_game_entry()
            .map(|e| e.active_manifest_id.as_str())
            .unwrap_or("");

        let latest = app
            .selected_game_entry()
            .map(|e| e.latest_manifest_id.as_str())
            .unwrap_or("");

        let items: Vec<ListItem> = cached
            .iter()
            .enumerate()
            .map(|(i, manifest_id)| {
                let is_active = manifest_id == active;
                let is_latest = manifest_id == latest;

                let user_label = app.manifest_db.get_label(manifest_id);
                let display = match user_label {
                    Some(lbl) => format!("{lbl}  {manifest_id}"),
                    None => manifest_id.clone(),
                };

                let label = match (is_active, is_latest) {
                    (true, true) => format!("● {display} (installed) (latest)"),
                    (true, false) => format!("● {display} (installed)"),
                    (false, true) => format!("  {display} (latest)"),
                    (false, false) => format!("  {display}"),
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
        f.render_widget(list, layout[1]);
    }

    if let VersionPickerMode::EditingLabel { input } = &app.version_picker_state.mode {
        let bar = Paragraph::new(format!(" Label: {}█", input))
            .style(theme::text());
        f.render_widget(bar, layout[2]);
    }

    let help_text = if editing {
        " [Enter] confirm   [Esc] cancel "
    } else {
        " [↑↓] select   [Enter] switch   [E] label   [Esc] cancel "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[3]);
}
