use crate::app::App;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Downgrade Game ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner.inner(Margin { horizontal: 1, vertical: 0 }));

    // SteamDB URL
    let url_block = Block::default()
        .title(" 1. Open this URL in your browser to find the manifest ID ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let url_para = Paragraph::new(app.wizard_state.steamdb_url.as_str())
        .style(Style::default().fg(Color::Cyan))
        .block(url_block);
    f.render_widget(url_para, layout[0]);

    // Manifest ID input
    let cursor = if !app.wizard_state.is_downloading { "█" } else { "" };
    let input_style = if app.wizard_state.is_downloading {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    };
    let input_block = Block::default()
        .title(" 2. Enter target manifest ID then press [Enter] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let input_para =
        Paragraph::new(format!("{}{}", app.wizard_state.manifest_input, cursor))
            .style(input_style)
            .block(input_block);
    f.render_widget(input_para, layout[1]);

    // Progress log
    let (log_title, log_border_style) = if app.wizard_state.error.is_some() {
        (" Error ", Style::default().fg(Color::Red))
    } else if app.wizard_state.is_downloading {
        (" Downloading... ", Style::default().fg(Color::Yellow))
    } else {
        (" Output ", Style::default().fg(Color::DarkGray))
    };

    let log_items: Vec<ListItem> = if let Some(err) = &app.wizard_state.error {
        vec![ListItem::new(err.as_str()).style(Style::default().fg(Color::Red))]
    } else {
        app.wizard_state
            .progress_lines
            .iter()
            .map(|l| ListItem::new(l.as_str()))
            .collect()
    };

    let log_block = Block::default()
        .title(log_title)
        .borders(Borders::ALL)
        .border_style(log_border_style);
    let log_list = List::new(log_items).block(log_block);
    f.render_widget(log_list, layout[2]);

    // Help line
    let help = Paragraph::new(" [O] open SteamDB in browser   [Esc] cancel ")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, layout[3]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
