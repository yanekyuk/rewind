use crate::app::{App, ReshadeSetupStep};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

const API_LABELS: &[(&str, &str)] = &[
    ("Dxgi", "DX10 / DX11 / DX12  (most games)"),
    ("D3d9", "DX9  (older games)"),
    ("OpenGl32", "OpenGL"),
    ("Vulkan1", "Vulkan"),
];

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(55, 50, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" ReShade Setup ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_accent());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = inner.inner(Margin { horizontal: 1, vertical: 0 });

    match app.reshade_state.step {
        ReshadeSetupStep::PickApi => draw_pick_api(f, app, content),
        ReshadeSetupStep::ConfirmShaders => draw_confirm_shaders(f, app, content),
        ReshadeSetupStep::Downloading => draw_downloading(f, app, content),
    }
}

fn draw_pick_api(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // prompt
            Constraint::Length(1), // spacer
            Constraint::Min(0),    // list
            Constraint::Length(1), // help
        ])
        .split(area);

    let prompt = Paragraph::new("Select graphics API:").style(theme::text());
    f.render_widget(prompt, layout[0]);

    let items: Vec<ListItem> = API_LABELS
        .iter()
        .enumerate()
        .map(|(i, (name, desc))| {
            let style = if i == app.reshade_state.selected_api {
                theme::selected()
            } else {
                theme::text()
            };
            ListItem::new(format!("  {}  —  {}", name, desc)).style(style)
        })
        .collect();

    let list = List::new(items);
    f.render_widget(list, layout[2]);

    let help = Paragraph::new(" [↑↓/jk] navigate  [Enter] select  [Esc] cancel ")
        .style(theme::help_bar());
    f.render_widget(help, layout[3]);
}

fn draw_confirm_shaders(f: &mut Frame, _app: &App, area: ratatui::layout::Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1), // help
        ])
        .split(area);

    let msg = Paragraph::new(
        "Download community shader pack?\n(reshade-shaders slim, ~10MB)\n\nPacks include common post-processing\npresets you can enable in-game.",
    )
    .style(theme::text());
    f.render_widget(msg, layout[0]);

    let help = Paragraph::new(" [Y] Yes  [N] No  [Esc] back ").style(theme::help_bar());
    f.render_widget(help, layout[1]);
}

fn draw_downloading(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // progress lines
            Constraint::Length(1), // help
        ])
        .split(area);

    let lines: Vec<ListItem> = app
        .reshade_state
        .lines
        .iter()
        .map(|l| ListItem::new(l.as_str()).style(theme::text()))
        .collect();

    if let Some(ref err) = app.reshade_state.error {
        let error_items: Vec<ListItem> = vec![
            ListItem::new(format!("Error: {}", err)).style(theme::status_error()),
            ListItem::new("Place ReShade64.dll manually in:").style(theme::text_secondary()),
            ListItem::new("  ~/.local/share/rewind/bin/").style(theme::text_secondary()),
        ];
        let list = List::new(error_items);
        f.render_widget(list, layout[0]);
    } else {
        let list = List::new(lines);
        f.render_widget(list, layout[0]);
    }

    let help_text = if app.reshade_state.done {
        " Done!  [Esc] close "
    } else if app.reshade_state.error.is_some() {
        " [Esc] close "
    } else {
        " Downloading... "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[1]);
}
