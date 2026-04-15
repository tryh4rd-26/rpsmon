use crate::app::App;
use crate::theme;
use crate::sort::SortBy;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

const RPS_BANNER: &str = r#" ███████████   ███████████   █████████ 
░░███░░░░░███ ░░███░░░░░███ ███░░░░░███
 ░███    ░███  ░███    ░███░███    ░░░ 
 ░██████████   ░██████████ ░░█████████ 
 ░███░░░░░███  ░███░░░░░░   ░░░░░░░░███
 ░███    ░███  ░███         ███    ░███
 █████   █████ █████       ░░█████████ 
░░░░░   ░░░░░ ░░░░░         ░░░░░░░░░  "#;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " System Insights ",
            Style::default()
                .fg(theme::BORDER_CPU)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_CPU));

    let pm = &app.process_manager;
    
    // Top CPU process
    let cpu_procs = pm.get_filtered_and_sorted_processes("", SortBy::Cpu);
    let top_cpu = if !cpu_procs.is_empty() {
        let p = cpu_procs[0];
        let cpu_count = pm.cpu_count() as f32;
        let cpu_pct = (p.cpu_usage() / cpu_count).min(100.0);
        format!("{}({:.0}%)", p.name(), cpu_pct)
    } else {
        "none".to_string()
    };
    
    // System wide metrics
    let total_threads = pm.get_total_threads();
    let open_fds = pm.get_open_fds();
    let ctx_switches = pm.get_context_switches();
    let interrupts = pm.get_interrupts();
    let zombie_count = pm.get_zombie_count();
    let daemon_count = pm.get_daemon_count();
    let avg_task_mem_mb = pm.get_avg_task_memory() as f64 / 1_048_576.0;
    let user_cpu_pct = pm.get_user_cpu_pct();
    let sys_cpu_pct = 100.0 - user_cpu_pct;

    let lines = vec![
        Line::from(vec![
            Span::styled("CPU Top: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(top_cpu, Style::default().fg(theme::BORDER_CPU)),
        ]),
        Line::from(vec![
            Span::styled("Threads: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{} ", total_threads), Style::default().fg(theme::TEXT_VALUE)),
            Span::styled("FDs: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{}", open_fds), Style::default().fg(theme::TEXT_VALUE)),
        ]),
        Line::from(vec![
            Span::styled("CtxSw: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{}/s ", ctx_switches), Style::default().fg(theme::TEXT_VALUE)),
            Span::styled("IRQ: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{}/s", interrupts), Style::default().fg(theme::TEXT_VALUE)),
        ]),
        Line::from(vec![
            Span::styled("Zombie: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{} ", zombie_count), Style::default().fg(theme::TEXT_DIM)),
            Span::styled("Daemon: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{}", daemon_count), Style::default().fg(theme::TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled("AvgMemTask: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("{:.1}M", avg_task_mem_mb), Style::default().fg(theme::TEXT_VALUE)),
        ]),
        Line::from(vec![
            Span::styled("CPU: ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(format!("User {:.0}% ", user_cpu_pct), Style::default().fg(theme::BORDER_CPU)),
            Span::styled(format!("Sys {:.0}%", sys_cpu_pct), Style::default().fg(theme::COLOR_ORANGE)),
        ]),
        // Separator
        Line::from(""),
        // Padding for banner
        Line::from(""),
    ];

    // Add banner with colors and centering
    let banner_width = 40; // Banner is approximately 40 chars wide
    let available_width = (area.width as i32 - 4).max(0) as u16; // Account for border/margin
    let center_padding = if available_width > banner_width { (available_width - banner_width) / 2 } else { 0 };
    
    let mut banner_lines: Vec<Line> = RPS_BANNER.lines()
        .map(|banner_line| {
            let mut line_spans = vec![];
            
            // Add centering padding
            if center_padding > 0 {
                line_spans.push(Span::styled(" ".repeat(center_padding as usize), Style::default().fg(theme::TEXT_DIM)));
            }
            
            // Add banner characters with colors
            for ch in banner_line.chars() {
                let color = match ch {
                    '█' => theme::BORDER_CPU,      // Green solid blocks
                    '░' => theme::COLOR_BLUE,      // Blue outline blocks
                    ' ' => theme::TEXT_DIM,        // Spaces
                    _ => theme::TEXT_LABEL,
                };
                line_spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
            }
            Line::from(line_spans)
        })
        .collect();

    let mut all_lines = lines;
    all_lines.extend(banner_lines);

    let paragraph = Paragraph::new(all_lines).block(block);
    f.render_widget(paragraph, area);
}
