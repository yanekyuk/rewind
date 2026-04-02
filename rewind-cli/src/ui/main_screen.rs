use crate::app::App;
use crate::ui::theme;
use rewind_core::steamdb;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use ratatui_image::StatefulImage;

pub fn draw(f: &mut Frame, app: &mut App) {
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

    // Title bar
    let title = Paragraph::new(" rewind — Steam Version Manager ")
        .style(theme::title());
    f.render_widget(title, outer[0]);

    // Content
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[1]);

    draw_game_list(f, app, content[0]);
    draw_detail_panel(f, app, content[1]);

    // Status bar
    let status = Paragraph::new(
        " [↑↓/jk] navigate  [D] download  [U] switch version  [O] SteamDB  [S] settings  [Q] quit ",
    )
    .style(theme::help_bar());
    f.render_widget(status, outer[2]);
}

fn draw_game_list(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .installed_games
        .iter()
        .enumerate()
        .map(|(i, game)| {
            let entry = app.games_config.games.iter().find(|e| e.app_id == game.app_id);
            let indicator = match entry {
                Some(e) if e.active_manifest_id != e.latest_manifest_id => "▼ ",
                Some(_) => "✓ ",
                None => "  ",
            };

            let style = if i == app.selected_game_index {
                theme::selected()
            } else {
                theme::text()
            };

            ListItem::new(format!("{}{}", indicator, game.name)).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" GAMES ")
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::border())
        .style(theme::base_bg());

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_detail_panel(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let Some(game) = app.selected_game() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(theme::border())
            .style(theme::base_bg());
        let inner = block.inner(area);
        f.render_widget(block, area);
        let msg = Paragraph::new(
            "No games found.\nPress [S] to add a Steam library.",
        )
        .style(theme::text_secondary());
        f.render_widget(msg, inner);
        return;
    };

    // Copy all data from game/entry before mutable borrow on image_state
    let game_app_id = game.app_id;
    let entry = app.games_config.games.iter().find(|e| e.app_id == game_app_id);

    let status_line = match entry {
        Some(e) if e.active_manifest_id != e.latest_manifest_id => "▼ Updates disabled",
        Some(e) if e.acf_locked => "✓ Updates disabled",
        Some(_) => "✓ Updates enabled",
        None => "  Updates enabled",
    };

    let active_manifest = entry
        .map(|e| e.active_manifest_id.clone())
        .unwrap_or_else(|| game.manifest_id.clone());

    let cached_list = entry
        .map(|e| e.cached_manifest_ids.join(", "))
        .unwrap_or_else(|| "none".into());

    let spoofed_line = match entry {
        Some(e) if e.active_manifest_id != e.latest_manifest_id => {
            format!("\n  Spoofed as: {}", e.latest_manifest_id)
        }
        _ => String::new(),
    };

    let text = format!(
        "  {name}\n  App ID:    {app_id}\n  Depot:     {depot_id}\n\n  Status:    {status}\n  Installed: {active}{spoofed}\n  Cached:    {cached}\n\n  [D] Download new version\n  [U] Switch version\n  [O] Open app on SteamDB",
        name = game.name,
        app_id = game.app_id,
        depot_id = game.depot_id,
        status = status_line,
        active = active_manifest,
        spoofed = spoofed_line,
        cached = cached_list,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::border())
        .style(theme::base_bg());

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Render hero image in the top portion if available, with text below.
    if let Some(protocol) = app.image_state.protocols.get_mut(&game_app_id) {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(inner);

        // Image fills its area with no additional margin
        let widget = StatefulImage::default();
        f.render_stateful_widget(widget, split[0], protocol);

        let para = Paragraph::new(text).wrap(Wrap { trim: false }).style(theme::text());
        f.render_widget(para, split[1]);
    } else {
        let para = Paragraph::new(text).wrap(Wrap { trim: false }).style(theme::text());
        f.render_widget(para, inner);
    }
}
