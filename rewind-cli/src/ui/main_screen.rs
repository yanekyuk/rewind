use crate::app::App;
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Wrap},
};
use ratatui_image::StatefulImage;

// Wide: 48+ columns inner width (6 lines tall)
const LOGO_WIDE: &str = "\
 ██████╗ ███████╗██╗    ██╗██╗███╗   ██╗██████╗\n\
 ██╔══██╗██╔════╝██║    ██║██║████╗  ██║██╔══██╗\n\
 ██████╔╝█████╗  ██║ █╗ ██║██║██╔██╗ ██║██║  ██║\n\
 ██╔══██╗██╔══╝  ██║███╗██║██║██║╚██╗██║██║  ██║\n\
 ██║  ██║███████╗╚███╔███╔╝██║██║ ╚████║██████╔╝\n\
 ╚═╝  ╚═╝╚══════╝ ╚══╝╚══╝ ╚═╝╚═╝  ╚═══╝╚═════╝";

// Medium: 28+ columns inner width (4 lines tall)
const LOGO_MEDIUM: &str = "\
 ╦═╗╔═╗╦ ╦╦╔╗╔╔╦╗\n\
 ╠╦╝║╣ ║║║║║║║ ║║\n\
 ╩╚═╚═╝╚╩╝╩╝╚╝═╩╝";

// Small: plain text fallback
const LOGO_SMALL: &str = "REWIND";

/// Returns the appropriate logo text and height (including border) for the given width.
fn logo_for_width(inner_width: u16) -> (&'static str, u16) {
    if inner_width >= 48 {
        (LOGO_WIDE, 8)   // 6 lines + 2 border
    } else if inner_width >= 20 {
        (LOGO_MEDIUM, 5)  // 3 lines + 2 border
    } else {
        (LOGO_SMALL, 3)   // 1 line + 2 border
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Content: left column (logo + games), right column (detail)
    let content = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(outer[0]);

    // Determine logo height based on available width (minus 2 for border)
    let logo_inner_width = content[0].width.saturating_sub(2);
    let (_, logo_height) = logo_for_width(logo_inner_width);

    // Left column: ASCII logo on top, games list below
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(logo_height), Constraint::Min(0)])
        .split(content[0]);

    draw_logo(f, left[0]);
    draw_game_list(f, app, left[1]);
    draw_detail_panel(f, app, content[1]);

    // Status bar
    let status = Paragraph::new(
        " [↑↓/jk] navigate  [D] download  [U] switch version  [O] SteamDB  [S] settings  [Q] quit ",
    )
    .style(theme::help_bar());
    f.render_widget(status, outer[1]);
}

fn draw_logo(f: &mut Frame, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let (logo_text, _) = logo_for_width(inner.width);
    let logo = Paragraph::new(logo_text)
        .style(theme::title())
        .alignment(Alignment::Center);
    f.render_widget(logo, inner);
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
        .border_style(theme::border());

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_detail_panel(f: &mut Frame, app: &mut App, area: ratatui::layout::Rect) {
    let Some(game) = app.selected_game() else {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Plain)
            .border_style(theme::border());
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
    let game_name = game.name.clone();
    let game_manifest_id = game.manifest_id.clone();
    let depot_id = game.depot_id;

    let entry = app.games_config.games.iter().find(|e| e.app_id == game_app_id);
    let entry_active_manifest = entry.map(|e| e.active_manifest_id.clone());
    let entry_latest_manifest = entry.map(|e| e.latest_manifest_id.clone());
    let entry_cached_ids = entry.map(|e| e.cached_manifest_ids.clone());

    // Populate launch options cache on first access for this appid.
    if !app.launch_options_cache.contains_key(&game_app_id) {
        let opts = rewind_core::scanner::find_launch_options(game_app_id);
        app.launch_options_cache.insert(game_app_id, opts);
    }

    let launch_line = match app.launch_options_cache.get(&game_app_id) {
        None => "\n  Launch:    \u{2026}".to_string(),
        Some(None) => String::new(),
        Some(Some(s)) => format!("\n  Launch:    {}", s),
    };

    let status_line = match (&entry_active_manifest, &entry_latest_manifest) {
        (Some(active), Some(latest)) if active != latest => "▼ Updates disabled",
        (Some(_), _) => "✓ Updates enabled",
        _ => "  Updates enabled",
    };

    let spoofed_line = match (&entry_active_manifest, &entry_latest_manifest) {
        (Some(active), Some(latest)) if active != latest => {
            format!("\n  Spoofed as: {}", latest)
        }
        _ => String::new(),
    };

    let active_manifest = entry_active_manifest
        .unwrap_or_else(|| game_manifest_id);

    let cached_list = entry_cached_ids
        .map(|ids| ids.join(", "))
        .unwrap_or_else(|| "none".into());

    let text = format!(
        "  {name}\n  App ID:    {app_id}\n  Depot:     {depot_id}\n\n  Status:    {status}\n  Installed: {active}{spoofed}\n  Cached:    {cached}{launch}\n\n  [D] Download new version\n  [U] Switch version\n  [O] Open app on SteamDB",
        name = game_name,
        app_id = game_app_id,
        depot_id = depot_id,
        status = status_line,
        active = active_manifest,
        spoofed = spoofed_line,
        cached = cached_list,
        launch = launch_line,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(theme::border());

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Render text at top; hero image below if available.
    if let Some(protocol) = app.image_state.protocols.get_mut(&game_app_id) {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(inner);

        let para = Paragraph::new(text).wrap(Wrap { trim: false }).style(theme::text());
        f.render_widget(para, split[0]);

        // Image fills its area with no additional margin
        let widget = StatefulImage::default();
        f.render_stateful_widget(widget, split[1], protocol);
    } else {
        let para = Paragraph::new(text).wrap(Wrap { trim: false }).style(theme::text());
        f.render_widget(para, inner);
    }
}
