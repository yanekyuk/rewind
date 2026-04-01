use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Margin, Rect},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

pub fn draw(f: &mut Frame, _app: &App) {
    let area = f.area();

    // Background fill
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(theme::base_bg()), area);

    let dialog_area = centered_rect(60, 14, area);
    f.render_widget(Clear, dialog_area);

    let block = Block::default()
        .title(" Welcome to rewind ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent())
        .style(theme::base_bg());

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

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(x, y, w, h)
}
