use crate::app::{App, FirstRunStep};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, app: &App) {
    match app.first_run_state.step {
        FirstRunStep::Welcome => draw_welcome(f),
        FirstRunStep::AccountPicker => draw_account_picker(f, app),
    }
}

fn draw_welcome(f: &mut Frame) {
    let area = f.area();
    let dialog_area = centered_rect(60, 14, area);
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Welcome to rewind ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let text = "rewind manages your Steam game versions.\n\n\
        It uses DepotDownloader to fetch previous versions\n\
        and switches between them instantly via symlinks.\n\n\
        Press [Enter] to configure your Steam libraries.\n\
        Press [Q] to quit.";

    let paragraph = Paragraph::new(text)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(theme::text());

    f.render_widget(paragraph, inner.inner(Margin { horizontal: 1, vertical: 1 }));
}

fn draw_account_picker(f: &mut Frame, app: &App) {
    let area = f.area();
    let accounts = &app.first_run_state.accounts;
    let height = (accounts.len() as u16 + 8).min(area.height);
    let dialog_area = centered_rect(60, height, area);
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Select Steam Account ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(dialog_area);
    f.render_widget(block, dialog_area);

    let inner_padded = inner.inner(Margin { horizontal: 1, vertical: 1 });

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(2),
        ])
        .split(inner_padded);

    // Account list — use the same selected-item style as the rest of the app
    let items: Vec<ListItem> = accounts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let label = format!("{} ({})", a.persona_name, a.account_name);
            let style = if i == app.first_run_state.selected_index {
                theme::list_item_selected()
            } else {
                theme::text()
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, layout[0]);

    // Footer
    let footer = Paragraph::new("[↑/↓] select   [Enter] confirm\nYou can change this later in Settings.")
        .alignment(Alignment::Center)
        .style(theme::text_secondary());
    f.render_widget(footer, layout[1]);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(x, y, w, h)
}
