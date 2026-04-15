use crate::app::App;
use crate::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_INFO))
        .style(Style::default().bg(theme::HEADER_BG));

    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .margin(1)
        .split(area);

    // Column 1: Host info
    let hostname = app.process_manager.hostname();
    let os = app.process_manager.os_version();
    let kernel = app.process_manager.kernel_version();
    let cpu_name = app.process_manager.get_cpu_name();
    let cpu_cores = app.process_manager.cpu_count();
    let total_ram_gb = app.process_manager.total_memory() as f64 / 1_073_741_824.0;

    let col1 = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("rps", Style::default().fg(theme::HEADER_TITLE).add_modifier(Modifier::BOLD)),
            Span::styled(" │ ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(&hostname, Style::default().fg(theme::TEXT_ACCENT)),
        ]),
        Line::from(vec![
            Span::styled("CPU: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{} ({} cores)", cpu_name, cpu_cores), Style::default().fg(theme::TEXT_VALUE)),
        ]),
        Line::from(vec![
            Span::styled("RAM: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{:.1} GB", total_ram_gb), Style::default().fg(theme::TEXT_VALUE)),
        ]),
    ]);
    f.render_widget(col1, inner[0]);

    // Column 2: OS + Uptime
    let uptime = app.process_manager.uptime();
    let up_d = uptime / 86400;
    let up_h = (uptime % 86400) / 3600;
    let up_m = (uptime % 3600) / 60;
    let uptime_str = if up_d > 0 {
        format!("{}d {}h {}m", up_d, up_h, up_m)
    } else {
        format!("{}h {}m", up_h, up_m)
    };

    let now = chrono_time();

    let col2 = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("OS: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{} ({})", os, kernel), Style::default().fg(theme::TEXT_VALUE)),
        ]),
        Line::from(vec![
            Span::styled("Up: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(uptime_str, Style::default().fg(Color::Rgb(120, 220, 180))),
        ]),
        Line::from(vec![
            Span::styled("Time: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(now, Style::default().fg(theme::TEXT_VALUE)),
        ]),
    ]);
    f.render_widget(col2, inner[1]);

    // Column 3: Load + Processes
    let load = sysinfo::System::load_average();
    let load1_col = theme::cpu_color(load.one as f32 * 10.0);
    let load5_col = theme::cpu_color(load.five as f32 * 10.0);
    let load15_col = theme::cpu_color(load.fifteen as f32 * 10.0);

    let procs = app.process_manager.get_all_processes();
    let total = procs.len();
    let mut running = 0u32;
    let mut sleeping = 0u32;
    let mut zombie = 0u32;
    let _total_threads: usize = procs.iter().map(|p| p.tasks().map(|t| t.len()).unwrap_or(1)).sum();

    for p in &procs {
        match p.status() {
            sysinfo::ProcessStatus::Run => running += 1,
            sysinfo::ProcessStatus::Zombie => zombie += 1,
            _ => sleeping += 1,
        }
    }

    let col3 = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Load: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{:.2}", load.one), Style::default().fg(load1_col)),
            Span::styled(" ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(format!("{:.2}", load.five), Style::default().fg(load5_col)),
            Span::styled(" ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(format!("{:.2}", load.fifteen), Style::default().fg(load15_col)),
        ]),
        Line::from(vec![
            Span::styled("Procs: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(total.to_string(), Style::default().fg(theme::TEXT_BRIGHT)),
            Span::styled(" (", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(format!("{}R ", running), Style::default().fg(theme::STATUS_RUN)),
            Span::styled(format!("{}S ", sleeping), Style::default().fg(theme::STATUS_SLEEP)),
            Span::styled(format!("{}Z", zombie), Style::default().fg(theme::STATUS_ZOMBIE)),
            Span::styled(")", Style::default().fg(theme::TEXT_DIM)),
        ]),
    ]);
    f.render_widget(col3, inner[2]);

    // Column 4: Hostname detail
    let cpu_brand = app.process_manager.get_cpu_brand();
    
    let col4 = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Brand: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(cpu_brand, Style::default().fg(theme::TEXT_VALUE)),
        ]),
    ]);
    f.render_widget(col4, inner[3]);
}

use ratatui::style::Color;

fn chrono_time() -> String {
    // Get system time without external chrono dependency
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = since_epoch.as_secs();
    // Adjust for IST (+5:30) since the user is in that timezone
    let ist_secs = secs + 5 * 3600 + 30 * 60;
    let h = (ist_secs % 86400) / 3600;
    let m = (ist_secs % 3600) / 60;
    let s = ist_secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn boot_time_str(uptime_secs: u64) -> String {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let boot_epoch = since_epoch.as_secs().saturating_sub(uptime_secs);
    let ist = boot_epoch + 5 * 3600 + 30 * 60;
    let h = (ist % 86400) / 3600;
    let m = (ist % 3600) / 60;
    format!("{:02}:{:02}", h, m)
}
