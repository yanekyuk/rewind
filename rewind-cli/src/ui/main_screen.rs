use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};
use ratatui_image::StatefulImage;
use rewind_core::steamdb;

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
        " [↑↓/jk] navigate  [D] downgrade  [U] upgrade  [L] lock  [O] SteamDB  [S] settings  [Q] quit ",
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

fn draw_detail_panel(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
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

    let entry = app.games_config.games.iter().find(|e| e.app_id == game.app_id);

    let status_line = match entry {
        Some(e) if e.acf_locked && e.active_manifest_id != e.latest_manifest_id => {
            "▼ Downgraded (locked)"
        }
        Some(e) if e.acf_locked => "✓ Up to date (locked)",
        Some(_) => "  Managed",
        None => "  Unmanaged",
    };

    let active_manifest = entry
        .map(|e| e.active_manifest_id.as_str())
        .unwrap_or(game.manifest_id.as_str());

    let cached_list = entry
        .map(|e| e.cached_manifest_ids.join(", "))
        .unwrap_or_else(|| "none".into());

    let steamdb_url = steamdb::depot_manifests_url(game.depot_id);

    let text = format!(
        "  {name}\n  App ID:  {app_id}\n  Depot:   {depot_id}\n\n  Status:  {status}\n  Active:  {active}\n  Cached:  {cached}\n\n  SteamDB: {url}\n\n  [D] Downgrade / switch version\n  [U] Upgrade / switch version\n  [L] Toggle ACF lock\n  [O] Open app on SteamDB",
        name = game.name,
        app_id = game.app_id,
        depot_id = game.depot_id,
        status = status_line,
        active = active_manifest,
        cached = cached_list,
        url = steamdb_url,
    );

    // Render hero image in the top portion if available, with text below.
    if let (Some(picker), Some(dyn_img)) = (
        app.image_picker.as_ref(),
        app.image_state.loaded_images.get(&game.app_id),
    ) {
        // Image takes top ~40% with zero margin, text panel with border below
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        let image_area = split[0];

        // Render image edge-to-edge (no border, no margin)
        let mut protocol = picker.new_resize_protocol(dyn_img.clone());
        let widget = StatefulImage::default();
        f.render_stateful_widget(widget, image_area, &mut protocol);

        // Text area with border
        let text_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(theme::border())
            .style(theme::base_bg());
        let text_inner = text_block.inner(split[1]);
        f.render_widget(text_block, split[1]);

        let para = Paragraph::new(text).wrap(Wrap { trim: false }).style(theme::text());
        f.render_widget(para, text_inner);
    } else {
        // No image available: render text in bordered panel.
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(theme::border())
            .style(theme::base_bg());
        let inner = block.inner(area);
        f.render_widget(block, area);

        let para = Paragraph::new(text).wrap(Wrap { trim: false }).style(theme::text());
        f.render_widget(para, inner);
    }
}
