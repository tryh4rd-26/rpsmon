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
            " Network ",
            Style::default()
                .fg(theme::BORDER_NET)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_NET));
    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(8),   // Stats + Interface IPs (Flexible)
            Constraint::Min(10),  // Guaranteed Graph Space
        ])
        .margin(1)
        .split(area);

    // Network stats
    let (rx_rate, tx_rate) = app.net_rate();
    let (rx_total, tx_total) = app.process_manager.get_network_stats();

    let mut lines = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(" ▼ Rx: ", Style::default().fg(theme::NET_RX).add_modifier(Modifier::BOLD)),
        Span::styled(format_bytes_rate(rx_rate), Style::default().fg(theme::NET_RX)),
        Span::styled("   ▲ Tx: ", Style::default().fg(theme::NET_TX).add_modifier(Modifier::BOLD)),
        Span::styled(format_bytes_rate(tx_rate), Style::default().fg(theme::NET_TX)),
    ]));

    lines.push(Line::from(vec![
        Span::styled(" Total Rx: ", Style::default().fg(theme::TEXT_LABEL)),
        Span::styled(format_bytes(rx_total), Style::default().fg(theme::TEXT_VALUE)),
        Span::styled("  Tx: ", Style::default().fg(theme::TEXT_LABEL)),
        Span::styled(format_bytes(tx_total), Style::default().fg(theme::TEXT_VALUE)),
    ]));

    // Interface list with IPs
    lines.push(Line::from(vec![
        Span::styled(" Interfaces:", Style::default().fg(theme::TEXT_LABEL)),
    ]));
    
    // Sort networks to find active ones first
    let mut nets: Vec<_> = app.process_manager.networks.iter().collect();
    nets.sort_by(|a, b| (b.1.received() + b.1.transmitted()).cmp(&(a.1.received() + a.1.transmitted())));

    for (name, data) in nets.into_iter().take(4) {
        let mut iface_line = vec![
            Span::styled(format!("  {:<6} ", name), Style::default().fg(theme::TEXT_ACCENT)),
        ];
        
        let in_out_pkts = data.packets_received() + data.packets_transmitted();
        iface_line.push(Span::styled(
            format!("(pkts: {}) ", in_out_pkts),
            Style::default().fg(theme::TEXT_DIM),
        ));
        
        // Robust IP matching (trim and compare)
        let clean_name = name.trim_matches(':').trim().to_string();
        if let Some(ips) = app.process_manager.iface_ips.get(&clean_name) {
            for ip in ips {
                iface_line.push(Span::styled(
                    format!("{} ", ip),
                    Style::default().fg(theme::TEXT_VALUE),
                ));
            }
        }
        
        lines.push(Line::from(iface_line));
    }

    let stats_widget = Paragraph::new(lines);
    f.render_widget(stats_widget, inner[0]);

    // Network smoothed area chart using Canvas
    let max_rx = app.net_history.iter().map(|&(r, _)| r).fold(0.0, f64::max) / 1024.0;
    let max_tx = app.net_history.iter().map(|&(_, t)| t).fold(0.0, f64::max) / 1024.0;
    let y_max = (max_rx.max(max_tx) * 1.2).max(10.0);

    let net_canvas = Canvas::default()
        .block(Block::default())
        .x_bounds([0.0, 60.0])
        .y_bounds([0.0, y_max])
        .paint(|ctx| {
            let rx_pts: Vec<(f64, f64)> = app.net_history.iter().enumerate()
                .map(|(i, &(r, _))| (i as f64, r as f64 / 1024.0)).collect();
            let tx_pts: Vec<(f64, f64)> = app.net_history.iter().enumerate()
                .map(|(i, &(_, t))| (i as f64, t as f64 / 1024.0)).collect();

            // 1. Draw Fills first with gradient shading
            for i in 0..rx_pts.len() {
                let rx = rx_pts[i];
                let tx = tx_pts[i];
                
                if rx.1 > 0.1 {
                    let steps = (rx.1 / 2.0).min(50.0) as i32;
                    for step in 0..steps {
                        let sy = (step as f64) * 2.0;
                        if sy >= rx.1 { break; }
                        let fade_factor = ((rx.1 - sy) / rx.1 * 0.7 + 0.2).min(1.0);
                        ctx.draw(&CanvasLine {
                            x1: rx.0, y1: sy,
                            x2: rx.0, y2: (sy + 2.0).min(rx.1),
                            color: theme::fade_color(theme::NET_RX, fade_factor as f32),
                        });
                    }
                }
                
                if tx.1 > 0.1 {
                    let steps = (tx.1 / 2.0).min(50.0) as i32;
                    for step in 0..steps {
                        let sy = (step as f64) * 2.0;
                        if sy >= tx.1 { break; }
                        let fade_factor = ((tx.1 - sy) / tx.1 * 0.7 + 0.2).min(1.0);
                        ctx.draw(&CanvasLine {
                            x1: tx.0, y1: sy,
                            x2: tx.0, y2: (sy + 2.0).min(tx.1),
                            color: theme::fade_color(theme::NET_TX, fade_factor as f32),
                        });
                    }
                }

                // 2. Draw Top Polylines
                if i > 0 {
                    let prev_rx = rx_pts[i-1];
                    let prev_tx = tx_pts[i-1];
                    ctx.draw(&CanvasLine {
                        x1: prev_rx.0, y1: prev_rx.1, x2: rx.0, y2: rx.1,
                        color: theme::NET_RX,
                    });
                    ctx.draw(&CanvasLine {
                        x1: prev_tx.0, y1: prev_tx.1, x2: tx.0, y2: tx.1,
                        color: theme::NET_TX,
                    });
                }
            }

            ctx.print(2.0, y_max * 0.9, Span::styled("▼ RX", Style::default().fg(theme::NET_RX)));
            ctx.print(2.0, y_max * 0.75, Span::styled("▲ TX", Style::default().fg(theme::NET_TX)));
        });

    f.render_widget(net_canvas, inner[1]);
}

fn format_bytes_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_073_741_824.0 {
        format!("{:.1} GB/s", bytes_per_sec / 1_073_741_824.0)
    } else if bytes_per_sec >= 1_048_576.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1} KB/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0} B/s", bytes_per_sec)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}
