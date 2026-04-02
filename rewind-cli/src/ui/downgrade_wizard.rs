use crate::app::{App, StepStatus};
use crate::ui::theme;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin},
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};

pub fn draw(f: &mut Frame, app: &App) {
    let area = crate::ui::centered_rect(70, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" Download New Version ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_focused())
;

    let inner = block.inner(area);
    f.render_widget(block, area);

    let content = inner.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });

    if app.wizard_state.is_downloading || !app.wizard_state.steps.is_empty() {
        draw_download_view(f, app, content);
    } else {
        draw_input_view(f, app, content);
    }
}

/// The initial view: guidance text, manifest input, output log, help line.
fn draw_input_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let warn_height: u16 = if app.wizard_state.steam_warning { 1 } else { 0 };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),           // guidance text
            Constraint::Length(warn_height), // steam warning (0 if not needed)
            Constraint::Length(3),           // manifest input
            Constraint::Min(0),              // error/output log
            Constraint::Length(1),           // help line
        ])
        .split(area);

    // Guidance text
    let guidance = Paragraph::new(
        " Find your target version:\n \
         [P] Patches  \u{2014} browse patch notes to find the version you want,\n \
                        note its date\n \
         [M] Manifests \u{2014} match the date to find the corresponding\n \
                        manifest ID",
    )
    .style(theme::text());
    f.render_widget(guidance, layout[0]);

    // Steam warning
    if app.wizard_state.steam_warning {
        let warn = Paragraph::new(" \u{26a0} Steam is running. Quit Steam before downloading.")
            .style(theme::status_warning());
        f.render_widget(warn, layout[1]);
    }

    // Manifest ID input
    let cursor = if !app.wizard_state.is_downloading {
        "\u{2588}"
    } else {
        ""
    };
    let input_style = if app.wizard_state.is_downloading {
        theme::input_inactive()
    } else {
        theme::input_active()
    };
    let input_block = Block::default()
        .title(" Manifest ID ")
        .borders(Borders::ALL)
        .border_style(theme::border_focused());
    let input_para =
        Paragraph::new(format!("{}{}", app.wizard_state.manifest_input, cursor))
            .style(input_style)
            .block(input_block);
    f.render_widget(input_para, layout[2]);

    // Error / output log
    let (log_title, log_border_style) = if app.wizard_state.error.is_some() {
        (" Error ", Style::default().fg(theme::ERROR))
    } else {
        (" Output ", theme::border())
    };

    let log_items: Vec<ListItem> = if let Some(err) = &app.wizard_state.error {
        vec![ListItem::new(err.as_str()).style(Style::default().fg(theme::ERROR))]
    } else {
        vec![]
    };

    let log_block = Block::default()
        .title(log_title)
        .borders(Borders::ALL)
        .border_style(log_border_style);
    let log_list = List::new(log_items).block(log_block);
    f.render_widget(log_list, layout[3]);

    // Help line
    let help_text = if app.wizard_state.error_url.is_some() {
        " [O] open download page   [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [P] patches   [M] manifests   [Enter] download   [Esc] cancel "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[4]);
}

/// The download-in-progress view: steps on top, DepotDownloader output below.
fn draw_download_view(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let step_count = app.wizard_state.steps.len() as u16;

    let prompt_height = if app.wizard_state.prompt_input.is_some() {
        3u16
    } else {
        0
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(step_count + 1), // steps + top margin
            Constraint::Min(5),                 // DepotDownloader pane
            Constraint::Length(prompt_height),   // credential input (0 if hidden)
            Constraint::Length(1),               // help line
        ])
        .split(area);

    // --- Steps ---
    let step_items: Vec<ListItem> = app
        .wizard_state
        .steps
        .iter()
        .map(|(kind, status)| {
            let (icon, style) = match status {
                StepStatus::Pending => (
                    "[ ]",
                    theme::text_secondary(),
                ),
                StepStatus::InProgress => (
                    "[\u{2026}]",
                    theme::status_warning(),
                ),
                StepStatus::Done => (
                    "[\u{2713}]",
                    theme::status_success(),
                ),
                StepStatus::Failed(_) => (
                    "[\u{2717}]",
                    theme::status_error(),
                ),
            };
            ListItem::new(format!(" {} {}", icon, kind.label())).style(style)
        })
        .collect();
    let step_list = List::new(step_items);
    f.render_widget(step_list, layout[0]);

    // --- DepotDownloader output pane ---
    let depot_block = Block::default()
        .title(" DepotDownloader ")
        .borders(Borders::ALL)
        .border_style(theme::border())
;

    let depot_inner_height = depot_block.inner(layout[1]).height as usize;
    let depot_items: Vec<ListItem> = app
        .wizard_state
        .depot_lines
        .iter()
        .rev()
        .take(depot_inner_height)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|l| ListItem::new(l.as_str()).style(theme::text_secondary()))
        .collect();

    let depot_list = List::new(depot_items).block(depot_block);
    f.render_widget(depot_list, layout[1]);

    // --- Credential prompt input (if active) ---
    if let Some(ref input) = app.wizard_state.prompt_input {
        let label = app
            .wizard_state
            .prompt_label
            .as_deref()
            .unwrap_or("Input required:");
        let is_password = label.to_lowercase().contains("password");
        let display_text = if is_password {
            format!("{}\u{2588}", "*".repeat(input.len()))
        } else {
            format!("{}\u{2588}", input)
        };
        let prompt_block = Block::default()
            .title(format!(" {} ", label))
            .borders(Borders::ALL)
            .border_style(theme::border_accent())
    ;
        let prompt_para = Paragraph::new(display_text)
            .style(theme::input_active())
            .block(prompt_block);
        f.render_widget(prompt_para, layout[2]);
    }

    // --- Help line ---
    let help_text = if app
        .wizard_state
        .error
        .as_ref()
        .map(|e| e.contains("[R]"))
        .unwrap_or(false)
    {
        " [R] restart in terminal   [Esc] cancel   [Ctrl+C] quit "
    } else {
        " [Esc] cancel   [Ctrl+C] quit "
    };
    let help = Paragraph::new(help_text).style(theme::help_bar());
    f.render_widget(help, layout[3]);
}
