use crate::app::App;
use crate::sort::SortBy;
use crate::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Block, BorderType, Borders, Paragraph,
    },
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " Memory ",
            Style::default()
                .fg(theme::BORDER_MEM)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_MEM));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // RAM bar
            Constraint::Length(1), // Swap bar
            Constraint::Length(6), // Memory type bars
            Constraint::Length(8), // Bigger graph
            Constraint::Min(5),    // Dense info stats
        ])
        .margin(1)
        .split(area);

    let pm = &app.process_manager;
    let total_mem = pm.total_memory();
    let used_mem = pm.used_memory();

    // ── RAM Bar ──
    let mem_pct = if total_mem > 0 {
        (used_mem as f64 / total_mem as f64) * 100.0
    } else {
        0.0
    };
    let used_gb = used_mem as f64 / 1_073_741_824.0;
    let ram_bar_width = inner[0].width.saturating_sub(15);
    let ram_bar = theme::gradient_bar(mem_pct as f32, ram_bar_width, theme::BORDER_MEM);
    let ram_composite = Line::from(
        [
            Span::styled(" RAM ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.1}G ", used_gb),
                Style::default().fg(theme::TEXT_VALUE),
            ),
        ]
        .into_iter()
        .chain(ram_bar.spans.into_iter())
        .collect::<Vec<_>>(),
    );
    f.render_widget(Paragraph::new(ram_composite), inner[0]);

    // ── Swap Bar ──
    let swap_total = pm.total_swap();
    let swap_used = pm.used_swap();
    let swap_pct = if swap_total > 0 {
        (swap_used as f64 / swap_total as f64) * 100.0
    } else {
        0.0
    };
    let swap_used_gb = swap_used as f64 / 1_073_741_824.0;
    let swap_total_gb = swap_total as f64 / 1_073_741_824.0;
    let swap_bar_width = inner[1].width.saturating_sub(20);
    let swap_bar = theme::gradient_bar(swap_pct as f32, swap_bar_width, theme::GAUGE_SWAP);
    let swap_composite = Line::from(
        [
            Span::styled(" SWP ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.1}G/{:.1}G ", swap_used_gb, swap_total_gb),
                Style::default().fg(theme::TEXT_VALUE),
            ),
        ]
        .into_iter()
        .chain(swap_bar.spans.into_iter())
        .collect::<Vec<_>>(),
    );
    f.render_widget(Paragraph::new(swap_composite), inner[1]);

    // ── Memory Type Bars ──
    let wired = pm.get_mem_wired();
    let compressed = pm.get_mem_compressed();
    let purgeable = pm.get_mem_purgeable();
    let app_mem = pm.get_mem_app();
    let anonymous = pm.get_mem_anonymous();
    let file_backed = pm.get_mem_file_backed();

    let bar_labels = vec![
        ("Wired", wired, theme::BORDER_CPU),
        ("Compressed", compressed, theme::COLOR_ORANGE),
        ("App", app_mem, theme::COLOR_BLUE),
        ("Purgeable", purgeable, theme::TEXT_BRIGHT),
        ("Anonymous", anonymous, theme::BORDER_MEM),
        ("File-backed", file_backed, theme::TEXT_DIM),
    ];

    let mut mem_type_lines = vec![];
    for (label, val, color) in bar_labels {
        let pct = if total_mem > 0 {
            (val as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        };
        let val_gb = val as f64 / 1_073_741_824.0;
        let bar_width = inner[2].width.saturating_sub(20);
        let bar = theme::gradient_bar(pct as f32, bar_width, color);

        let mut line_spans = vec![
            Span::styled(
                format!(" {} ", label),
                Style::default().fg(theme::TEXT_LABEL),
            ),
            Span::styled(
                format!("{:.1}G ", val_gb),
                Style::default().fg(theme::TEXT_VALUE),
            ),
        ];
        line_spans.extend(bar.spans.iter().cloned());
        mem_type_lines.push(Line::from(line_spans));
    }
    f.render_widget(Paragraph::new(mem_type_lines), inner[2]);

    // ── Memory Graph - BIGGER (8 lines) with SMALLER SCALE ──
    let mem_canvas = Canvas::default()
        .block(Block::default())
        .x_bounds([0.0, 60.0])
        .y_bounds([-25.0, 25.0]) // Even smaller scale for extreme detail
        .paint(|ctx| {
            let history: Vec<(f64, f64)> = app
                .mem_history
                .iter()
                .enumerate()
                .map(|(i, &v)| {
                    let centered = (v as f64) - 50.0;
                    (i as f64, centered.max(-25.0).min(25.0))
                })
                .collect();

            if history.len() < 2 {
                return;
            }

            for &(x, y) in &history {
                if y.abs() > 0.5 {
                    let steps = (y.abs() / 2.0).min(50.0) as i32;
                    for step in 0..steps {
                        let sy = (step as f64) * 2.0;
                        if sy >= y.abs() {
                            break;
                        }
                        let fade_factor = ((y.abs() - sy) / y.abs() * 0.7 + 0.2).min(1.0);
                        let normalized = ((y.abs() + 25.0) / 50.0 * 100.0).min(100.0);
                        let line_color = theme::mem_color(normalized as f32);

                        let y1 = if y >= 0.0 { sy } else { -sy };
                        let y2 = if y >= 0.0 {
                            (sy + 2.0).min(y)
                        } else {
                            -(sy + 2.0).min(y.abs())
                        };

                        ctx.draw(&CanvasLine {
                            x1: x,
                            y1,
                            x2: x,
                            y2,
                            color: theme::fade_color(line_color, fade_factor as f32),
                        });
                    }
                }
            }

            for i in 0..history.len() - 1 {
                let (x1, y1) = history[i];
                let (x2, y2) = history[i + 1];
                let mid_val = (y1 + y2) / 2.0;
                let normalized = ((mid_val + 25.0) / 50.0 * 100.0).min(100.0);
                let line_color = theme::mem_color(normalized as f32);
                ctx.draw(&CanvasLine {
                    x1,
                    y1,
                    x2,
                    y2,
                    color: line_color,
                });
                ctx.draw(&CanvasLine {
                    x1,
                    y1: -y1,
                    x2,
                    y2: -y2,
                    color: line_color,
                });
            }
        });

    f.render_widget(mem_canvas, inner[3]);

    // ── Dense Info Block ──
    let available_mem = pm.available_memory();
    let free_mem = pm.free_memory();
    let cache_mem = pm.get_mem_cache();

    let avail_gb = available_mem as f64 / 1_073_741_824.0;
    let free_gb = free_mem as f64 / 1_073_741_824.0;
    let cache_mb = cache_mem as f64 / 1_048_576.0;

    let processes = pm.get_filtered_and_sorted_processes("", SortBy::Memory);
    let top_mem_proc = if !processes.is_empty() {
        format!(
            "{}({:.0}%)",
            processes[0].name(),
            (processes[0].memory() as f64 / total_mem as f64 * 100.0)
        )
    } else {
        "none".to_string()
    };

    let info_lines = vec![
        Line::from(vec![
            Span::styled("Used: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.0}% ", mem_pct),
                Style::default().fg(theme::mem_color(mem_pct as f32)),
            ),
            Span::styled("Avail: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.1}G ", avail_gb),
                Style::default().fg(theme::TEXT_VALUE),
            ),
            Span::styled("Free: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.1}G ", free_gb),
                Style::default().fg(theme::TEXT_BRIGHT),
            ),
        ]),
        Line::from(vec![
            Span::styled("Cache: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.0}M ", cache_mb),
                Style::default().fg(theme::COLOR_BLUE),
            ),
            Span::styled("Top: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(top_mem_proc, Style::default().fg(theme::TEXT_ACCENT)),
        ]),
    ];

    let paragraph = Paragraph::new(info_lines);
    f.render_widget(paragraph, inner[4]);
}
