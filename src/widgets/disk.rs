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
            " Disk ",
            Style::default()
                .fg(theme::BORDER_DISK)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_DISK));

    let pm = &app.process_manager;
    let (read_rate, write_rate) = app.disk_io_rate();

    let mut lines = vec![];

    // I/O Activity Header
    lines.push(Line::from(vec![
        Span::styled("I/O ", Style::default().fg(theme::TEXT_LABEL).add_modifier(Modifier::BOLD)),
        Span::styled("▲", Style::default().fg(theme::NET_RX)),
        Span::styled(format_rate(read_rate), Style::default().fg(theme::NET_RX)),
        Span::raw("  "),
        Span::styled("▼", Style::default().fg(theme::NET_TX)),
        Span::styled(format_rate(write_rate), Style::default().fg(theme::NET_TX)),
    ]));
    lines.push(Line::from(""));

    // Disk list - hierarchical display with all partition details
    let disks = pm.get_disks();
    
    if disks.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("No disks found", Style::default().fg(theme::TEXT_DIM)),
        ]));
    } else {
        for (idx, disk) in disks.iter().enumerate() {
            if idx >= 10 { break; } // Limit to 10 partitions
            
            let total = disk.total_space;
            if total == 0 { continue; } // Skip empty/invalid
            
            let mount = disk.mount_point.to_string_lossy();
            let used = disk.used_space;
            let pct = ((used as f64 / total as f64) * 100.0) as f32;
            
            let total_gib = total as f64 / 1_073_741_824.0;
            let used_gib = used as f64 / 1_073_741_824.0;

            // Extract mount point name (last component)
            let mount_name = if mount == "/" {
                "root".to_string()
            } else {
                mount.split('/').last().unwrap_or(&mount).to_string()
            };

            // Partition name line
            lines.push(Line::from(vec![
                Span::styled(
                    format!("▸ {}", mount_name),
                    Style::default()
                        .fg(theme::TEXT_ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}%", (pct as u32)),
                    Style::default().fg(theme::disk_color(pct)),
                ),
            ]));

            // Metrics line: used/total
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{:.1}G", used_gib),
                    Style::default().fg(theme::TEXT_VALUE),
                ),
                Span::styled(" / ", Style::default().fg(theme::TEXT_DIM)),
                Span::styled(
                    format!("{:.1}G", total_gib),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]));

            // Usage bar
            let bar = theme::gradient_bar(pct, 12u16, theme::BORDER_DISK);
            let mut bar_line = vec![Span::raw("  ")];
            bar_line.extend(bar);
            lines.push(Line::from(bar_line));
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_073_741_824.0 {
        format!("{:.2}G/s", bytes_per_sec / 1_073_741_824.0)
    } else if bytes_per_sec >= 1_048_576.0 {
        format!("{:.1}M/s", bytes_per_sec / 1_048_576.0)
    } else if bytes_per_sec >= 1024.0 {
        format!("{:.1}K/s", bytes_per_sec / 1024.0)
    } else {
        format!("{:.0}B/s", bytes_per_sec)
    }
}  
