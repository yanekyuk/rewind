use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = f.area();

    // Background fill
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(theme::base_bg()), area);

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
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(content);

    // Username input
    let username_focused = app.settings_state.focused_field == 0;
    let username_border_style = if username_focused {
        theme::border_focused()
    } else {
        theme::border()
    };
    let username_text_style = if username_focused {
        theme::input_active()
    } else {
        theme::input_inactive()
    };
    let cursor = if username_focused { "█" } else { "" };
    let username_block = Block::default()
        .title(" Steam Username ")
        .borders(Borders::ALL)
        .border_style(username_border_style)
        .style(theme::base_bg());
    let username_para =
        Paragraph::new(format!("{}{}", app.settings_state.username_input, cursor))
            .style(username_text_style)
            .block(username_block);
    f.render_widget(username_para, sections[0]);

    // Library path input
    let library_focused = app.settings_state.focused_field == 1;
    let library_border_style = if library_focused {
        theme::border_focused()
    } else {
        theme::border()
    };
    let library_text_style = if library_focused {
        theme::input_active()
    } else {
        theme::input_inactive()
    };
    let lib_cursor = if library_focused { "█" } else { "" };
    let library_block = Block::default()
        .title(" Add Steam Library Path (Enter to add) ")
        .borders(Borders::ALL)
        .border_style(library_border_style)
        .style(theme::base_bg());
    let library_para =
        Paragraph::new(format!("{}{}", app.settings_state.library_input, lib_cursor))
            .style(library_text_style)
            .block(library_block);
    f.render_widget(library_para, sections[2]);

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
        .border_style(theme::border())
        .style(theme::base_bg());

    if lib_items.is_empty() {
        let msg = Paragraph::new("  No libraries configured yet.")
            .style(theme::text_secondary())
            .block(lib_list_block);
        f.render_widget(msg, sections[3]);
    } else {
        let list = List::new(lib_items).block(lib_list_block);
        f.render_widget(list, sections[3]);
    }

    // Status bar
    let help = Paragraph::new(
        " [Tab] switch field   [Enter] save/add   [Esc] back ",
    )
    .style(theme::help_bar());
    f.render_widget(help, outer[2]);
}
