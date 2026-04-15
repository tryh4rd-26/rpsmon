use crate::app::App;
use crate::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{canvas::{Canvas, Line as CanvasLine}, Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " CPU ",
            Style::default()
                .fg(theme::BORDER_CPU)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_CPU));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),    // Per-core bars (compacted)
            Constraint::Length(1),    // Heatmap row
            Constraint::Min(14),      // Maximized chart area
        ])
        .margin(1)
        .split(area);

    draw_core_bars(f, app, inner[0]);
    draw_heatmap(f, app, inner[1]);
    draw_cpu_chart(f, app, inner[2]);
}

fn draw_core_bars(f: &mut Frame, app: &App, area: Rect) {
    let cpu_usages = app.process_manager.get_cpu_usages();
    if cpu_usages.is_empty() {
        return;
    }

    let half = (cpu_usages.len() + 1) / 2;
    let max_rows = area.height as usize;

    let mut lines = Vec::new();
    for i in 0..half.min(max_rows) {
        let mut spans = Vec::new();
        let left_usage = cpu_usages.get(i).copied().unwrap_or(0.0);
        spans.extend(format_core_bar(i, left_usage, 8)); // Shorter bars to save space

        if i + half < cpu_usages.len() {
            let right_idx = i + half;
            let right_usage = cpu_usages.get(right_idx).copied().unwrap_or(0.0);
            spans.push(Span::styled(" │ ", Style::default().fg(theme::TEXT_DIM)));
            spans.extend(format_core_bar(right_idx, right_usage, 8));
        }
        lines.push(Line::from(spans));
    }

    let widget = Paragraph::new(lines);
    f.render_widget(widget, area);
}

fn format_core_bar<'a>(idx: usize, usage: f32, width: usize) -> Vec<Span<'a>> {
    let bar = theme::gradient_bar(usage, width as u16, theme::cpu_color(usage));
    let color = theme::cpu_color(usage);

    let mut spans = vec![
        Span::styled(
            format!("C{:<2}", idx),
            Style::default().fg(theme::TEXT_LABEL),
        ),
        Span::styled("[", Style::default().fg(theme::TEXT_DIM)),
    ];
    spans.extend(bar.spans);
    spans.push(Span::styled("]", Style::default().fg(theme::TEXT_DIM)));
    spans.push(Span::styled(
        format!("{:>4.0}%", usage),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    spans
}

fn draw_heatmap(f: &mut Frame, app: &App, area: Rect) {
    let cpu_usages = app.process_manager.get_cpu_usages();
    let mut spans = vec![Span::styled(" Cores: ", Style::default().fg(theme::TEXT_LABEL))];

    for (i, &usage) in cpu_usages.iter().enumerate() {
        let color = theme::heatmap_color(usage);
        spans.push(Span::styled("▇", Style::default().fg(color)));
        if i < cpu_usages.len() - 1 {
            spans.push(Span::raw(" "));
        }
    }

    let global = app.process_manager.global_cpu_usage();
    spans.push(Span::styled(" Avg: ", Style::default().fg(theme::TEXT_LABEL)));
    spans.push(Span::styled(
        format!("{:.1}%", global),
        Style::default()
            .fg(theme::cpu_color(global))
            .add_modifier(Modifier::BOLD),
    ));

    let line = Paragraph::new(Line::from(spans));
    f.render_widget(line, area);
}

fn draw_cpu_chart(f: &mut Frame, app: &App, area: Rect) {
    let cpu_canvas = Canvas::default()
        .block(Block::default())
        .x_bounds([0.0, 60.0])
        .y_bounds([-100.0, 100.0])
        .paint(|ctx| {
            let history: Vec<(f64, f64)> = app.cpu_history.iter().enumerate()
                .map(|(i, &v)| (i as f64, (v as f64).min(100.0))).collect();

            if history.len() < 2 { return; }

            let _base_color = theme::BORDER_CPU;

            // Draw mirrored fill (above and below center line)
            for &(x, y) in &history {
                if y > 0.5 {
                    // Top half (positive)
                    let steps = (y / 2.0).min(50.0) as i32;
                    for step in 0..steps {
                        let sy = (step as f64) * 2.0;
                        if sy >= y { break; }
                        let fade_factor = ((y - sy) / y * 0.7 + 0.2).min(1.0);
                        let line_color = theme::cpu_color((sy + y / 2.0).max(0.0) as f32);
                        ctx.draw(&CanvasLine {
                            x1: x, y1: sy,
                            x2: x, y2: (sy + 2.0).min(y),
                            color: theme::fade_color(line_color, fade_factor as f32),
                        });
                    }
                    
                    // Bottom half (mirrored/negative)
                    let steps = (y / 2.0).min(50.0) as i32;
                    for step in 0..steps {
                        let sy = (step as f64) * 2.0;
                        if sy >= y { break; }
                        let fade_factor = ((y - sy) / y * 0.7 + 0.2).min(1.0);
                        let line_color = theme::cpu_color((sy + y / 2.0).max(0.0) as f32);
                        ctx.draw(&CanvasLine {
                            x1: x, y1: -sy,
                            x2: x, y2: -(sy + 2.0).min(y),
                            color: theme::fade_color(line_color, fade_factor as f32),
                        });
                    }
                }
            }

            // Draw top line with gradient coloring
            for i in 0..history.len() - 1 {
                let (x1, y1) = history[i];
                let (x2, y2) = history[i+1];
                let mid_val = (y1 + y2) / 2.0;
                let line_color = theme::cpu_color(mid_val as f32);
                ctx.draw(&CanvasLine { x1, y1, x2, y2, color: line_color });
            }
            
            // Draw bottom line (mirror)
            for i in 0..history.len() - 1 {
                let (x1, y1) = history[i];
                let (x2, y2) = history[i+1];
                let mid_val = (y1 + y2) / 2.0;
                let line_color = theme::cpu_color(mid_val as f32);
                ctx.draw(&CanvasLine { x1: x1, y1: -y1, x2: x2, y2: -y2, color: line_color });
            }
            
            ctx.print(2.0, 92.0, Span::styled("CPU %", Style::default().fg(theme::TEXT_LABEL)));
        });

    f.render_widget(cpu_canvas, area);
}
