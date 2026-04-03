use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Title
    let title = Paragraph::new(" Settings ")
        .alignment(Alignment::Center)
        .style(theme::title());
    f.render_widget(title, outer[0]);

    let content = outer[1].inner(Margin { horizontal: 2, vertical: 1 });

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // [0] username
            Constraint::Length(1),  // [1] spacer
            Constraint::Length(3),  // [2] library path input
            Constraint::Length(1),  // [3] spacer
            Constraint::Length(3),  // [4] steam account
            Constraint::Min(0),     // [5] libraries list
        ])
        .split(content);

    // Username input
    let username_focused = app.settings_state.focused_field == 0;
    let username_block = Block::default()
        .title(" Steam Username ")
        .borders(Borders::ALL)
        .border_style(if username_focused { theme::border_focused() } else { theme::border() });
    let cursor = if username_focused { "█" } else { "" };
    let username_para =
        Paragraph::new(format!("{}{}", app.settings_state.username_input, cursor))
            .style(if username_focused { theme::input_active() } else { theme::input_inactive() })
            .block(username_block);
    f.render_widget(username_para, sections[0]);

    // Library path input
    let library_focused = app.settings_state.focused_field == 1;
    let library_block = Block::default()
        .title(" Add Steam Library Path (Enter to add) ")
        .borders(Borders::ALL)
        .border_style(if library_focused { theme::border_focused() } else { theme::border() });
    let lib_cursor = if library_focused { "█" } else { "" };
    let library_para =
        Paragraph::new(format!("{}{}", app.settings_state.library_input, lib_cursor))
            .style(if library_focused { theme::input_active() } else { theme::input_inactive() })
            .block(library_block);
    f.render_widget(library_para, sections[2]);

    // Steam Account selector
    let account_focused = app.settings_state.focused_field == 2
        && !app.settings_state.available_accounts.is_empty();
    let account_label = if app.settings_state.available_accounts.is_empty() {
        "Auto (most recent)".to_string()
    } else if app.settings_state.account_index == 0 {
        "◀  Auto (most recent)  ▶".to_string()
    } else {
        let acct = &app.settings_state.available_accounts[app.settings_state.account_index - 1];
        format!("◀  {} ({})  ▶", acct.persona_name, acct.account_name)
    };
    let account_block = Block::default()
        .title(" Steam Account ")
        .borders(Borders::ALL)
        .border_style(if account_focused { theme::border_focused() } else { theme::border() });
    let account_para = Paragraph::new(account_label)
        .style(if account_focused { theme::input_active() } else { theme::input_inactive() })
        .block(account_block);
    f.render_widget(account_para, sections[4]);

    // Library list
    let lib_items: Vec<ListItem> = app
        .config
        .libraries
        .iter()
        .map(|l| ListItem::new(format!("  {}", l.path.display())).style(theme::text()))
        .collect();
    let lib_list_block = Block::default()
        .title(" Configured Libraries ")
        .borders(Borders::ALL)
        .border_style(theme::border());
    if lib_items.is_empty() {
        let msg = Paragraph::new("  No libraries configured yet.")
            .style(theme::text_secondary())
            .block(lib_list_block);
        f.render_widget(msg, sections[5]);
    } else {
        let list = List::new(lib_items).block(lib_list_block);
        f.render_widget(list, sections[5]);
    }

    // Status bar
    let help_text = if app.settings_state.available_accounts.is_empty() {
        " [Tab] switch field   [Enter] save/add   [Esc] back "
    } else {
        " [Tab] switch field   [←/→] change account   [Enter] save/add   [Esc] back "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, outer[2]);
}
