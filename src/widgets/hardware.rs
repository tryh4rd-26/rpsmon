use crate::app::App;
use crate::theme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " Hardware ",
            Style::default()
                .fg(theme::BORDER_HW)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_HW));

    let temp = app.process_manager.get_thermal();
    let batt = app.process_manager.get_battery();
    let gpu = app.process_manager.get_gpu();

    // Get component temperatures
    let mut component_temps: Vec<(String, f32)> = Vec::new();
    for component in &app.process_manager.components {
        let label = component.label().to_string();
        let t = component.temperature();
        if t > 0.0 {
            component_temps.push((label, t));
        }
    }
    component_temps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    component_temps.truncate(6); // Show top 6

    let mut lines = Vec::new();
    
    // Header
    lines.push(Line::from(vec![
        Span::styled(" [Thermal]", Style::default().fg(theme::TEXT_SECTION).add_modifier(Modifier::BOLD)),
    ]));

    // Per-component temps
    if component_temps.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("  CPU: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.1}°C", temp),
                Style::default()
                    .fg(theme::temp_color(temp))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
    } else {
        for (label, t) in &component_temps {
            // Truncate label to fit
            let short_label: String = label.chars().take(18).collect();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", short_label),
                    Style::default().fg(theme::TEXT_LABEL),
                ),
                Span::styled(
                    format!("{:.1}°C", t),
                    Style::default()
                        .fg(theme::temp_color(*t))
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }

    lines.push(Line::from(""));

    // Battery
    let batt_pct: f32 = batt
        .trim_end_matches('%')
        .parse()
        .unwrap_or(100.0);
    
    let bar_width: u16 = 10;
    let bar = theme::gradient_bar(batt_pct, bar_width, theme::battery_color(batt_pct));

    let mut batt_line = vec![
        Span::styled(" Batt: ", Style::default().fg(theme::TEXT_LABEL)),
        Span::styled("[", Style::default().fg(theme::TEXT_DIM)),
    ];
    batt_line.extend(bar.spans);
    batt_line.extend(vec![
        Span::styled("] ", Style::default().fg(theme::TEXT_DIM)),
        Span::styled(
            format!("{}", batt),
            Style::default()
                .fg(theme::battery_color(batt_pct))
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    lines.push(Line::from(batt_line));

    // GPU
    lines.push(Line::from(vec![
        Span::styled(" GPU: ", Style::default().fg(theme::TEXT_LABEL)),
        Span::styled(
            gpu,
            Style::default().fg(theme::TEXT_VALUE),
        ),
    ]));

    // Throttle indicator
    let throttle = if temp > 90.0 {
        ("THROTTLED", theme::STATUS_ZOMBIE)
    } else if temp > 80.0 {
        ("WARM", theme::cpu_color(80.0))
    } else {
        ("NORMAL", theme::STATUS_RUN)
    };

    lines.push(Line::from(vec![
        Span::styled(" Thermal: ", Style::default().fg(theme::TEXT_LABEL)),
        Span::styled(
            throttle.0,
            Style::default()
                .fg(throttle.1)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    let widget = Paragraph::new(lines).block(block);
    f.render_widget(widget, area);
}
