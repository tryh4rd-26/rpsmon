use crate::app::{App, AppMode};
use crate::theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs, Wrap},
    Frame,
};
use std::process::Command;

const TAB_NAMES: [&str; 6] = ["Identity", "Resources", "I/O", "Conn", "Relations", "Env"];

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            " Process Details ",
            Style::default()
                .fg(theme::BORDER_DETAIL)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_DETAIL));

    let processes = app
        .process_manager
        .get_filtered_and_sorted_processes(&app.search_query, app.sort_by);

    let pid = match app.selected_pid {
        Some(pid) => pid,
        None => {
            let p = Paragraph::new("\n No process selected.")
                .block(block)
                .style(Style::default().fg(theme::TEXT_DIM));
            f.render_widget(p, area);
            return;
        }
    };

    let proc = match processes.iter().find(|p| p.pid().as_u32() == pid) {
        Some(p) => *p,
        None => {
            let p = Paragraph::new("\n Process not found.")
                .block(block)
                .style(Style::default().fg(theme::TEXT_DIM));
            f.render_widget(p, area);
            return;
        }
    };

    if app.mode == AppMode::SignalMenu {
        draw_signal_menu(f, app, proc, area);
        return;
    }

    f.render_widget(block, area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Tabs
            Constraint::Min(4),    // Tab content
            Constraint::Length(2), // Actions bar
        ])
        .margin(1)
        .split(area);

    // Tab bar
    let tab_titles: Vec<Line> = TAB_NAMES
        .iter()
        .map(|t| Line::from(Span::raw(*t)))
        .collect();

    let tabs = Tabs::new(tab_titles)
        .select(app.detail_tab)
        .style(Style::default().fg(theme::TAB_INACTIVE))
        .highlight_style(
            Style::default()
                .fg(theme::TAB_ACTIVE)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::styled(" │ ", Style::default().fg(theme::TEXT_DIM)));

    f.render_widget(tabs, inner[0]);

    // Tab content
    match app.detail_tab {
        0 => draw_identity_tab(f, app, proc, inner[1]),
        1 => draw_resources_tab(f, app, proc, inner[1]),
        2 => draw_io_tab(f, proc, inner[1]),
        3 => draw_connections_tab(f, proc, inner[1]),
        4 => draw_relations_tab(f, app, proc, &processes, inner[1]),
        5 => draw_env_tab(f, proc, inner[1]),
        _ => {}
    }

    // Actions bar
    let actions = Paragraph::new(Line::from(vec![
        Span::styled(" Actions: ", Style::default().fg(theme::TEXT_LABEL)),
        Span::styled(
            "[v]",
            Style::default()
                .fg(theme::STATUS_ZOMBIE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Term  ", Style::default().fg(theme::TEXT_VALUE)),
        Span::styled(
            "[9]",
            Style::default()
                .fg(theme::STATUS_ZOMBIE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Kill  ", Style::default().fg(theme::TEXT_VALUE)),
        Span::styled(
            "[s]",
            Style::default()
                .fg(theme::TEXT_ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Signal  ", Style::default().fg(theme::TEXT_VALUE)),
        Span::styled(
            "[p]",
            Style::default()
                .fg(theme::STATUS_STOP)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Pause  ", Style::default().fg(theme::TEXT_VALUE)),
        Span::styled(
            "[Tab]",
            Style::default()
                .fg(theme::TEXT_DIM)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Tab", Style::default().fg(theme::TEXT_VALUE)),
    ]));
    f.render_widget(actions, inner[2]);
}

fn draw_identity_tab(f: &mut Frame, app: &App, proc: &sysinfo::Process, area: Rect) {
    let exe = proc
        .exe()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    let user_name = proc
        .user_id()
        .and_then(|uid| {
            app.process_manager
                .users
                .get_user_by_id(uid)
                .map(|u| u.name().to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let parent_pid = proc
        .parent()
        .map(|p| p.as_u32().to_string())
        .unwrap_or_else(|| "0".to_string());
    let cmd_str = proc.cmd().join(" ");
    let status = format!("{:?}", proc.status());
    let gid = proc
        .group_id()
        .map(|g| g.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let sid = proc
        .session_id()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let start_time = format_unix_time(proc.start_time());

    let mut lines = Vec::new();

    // Two columns for basic stats
    let col1_width = (area.width / 2).saturating_sub(2);

    lines.push(Line::from(vec![
        Span::styled(
            format!(
                "  Name:   {:<width$}",
                proc.name(),
                width = col1_width as usize - 10
            ),
            Style::default()
                .fg(theme::TEXT_VALUE)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" PID:    {}", proc.pid()),
            Style::default().fg(theme::TEXT_VALUE),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            format!(
                "  Status: {:<width$}",
                status,
                width = col1_width as usize - 10
            ),
            Style::default().fg(theme::status_color(proc.status())),
        ),
        Span::styled(
            format!(" User:   {}", user_name),
            Style::default().fg(theme::TEXT_ACCENT),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            format!(
                "  Parent: {:<width$}",
                parent_pid,
                width = col1_width as usize - 10
            ),
            Style::default().fg(theme::TEXT_VALUE),
        ),
        Span::styled(
            format!(" Start:  {}", start_time),
            Style::default().fg(theme::TEXT_VALUE),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            format!(
                "  GID:    {:<width$}",
                gid,
                width = col1_width as usize - 10
            ),
            Style::default().fg(theme::TEXT_VALUE),
        ),
        Span::styled(
            format!(" SID:    {}", sid),
            Style::default().fg(theme::TEXT_VALUE),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(section_header("Paths & Command"));
    lines.push(info_line("Executable", exe));

    let cmd_display = if cmd_str.is_empty() {
        "N/A".to_string()
    } else {
        cmd_str
    };
    lines.push(Line::from(vec![Span::styled(
        "  Command:  ",
        Style::default().fg(theme::TEXT_LABEL),
    )]));

    for chunk in cmd_display.as_bytes().chunks(area.width as usize - 6) {
        lines.push(Line::from(Span::styled(
            format!("    {}", String::from_utf8_lossy(chunk)),
            Style::default().fg(theme::TEXT_VALUE),
        )));
    }

    let widget = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}

fn draw_env_tab(f: &mut Frame, proc: &sysinfo::Process, area: Rect) {
    let mut lines = vec![section_header("Environment Variables")];
    let env = proc.environ();
    if env.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No environment variables visible.",
            Style::default().fg(theme::TEXT_DIM),
        )));
    } else {
        for var in env.iter().take(50) {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", var),
                Style::default().fg(theme::TEXT_VALUE),
            )]));
        }
        if env.len() > 50 {
            lines.push(Line::from(Span::styled(
                format!("  ... and {} more", env.len() - 50),
                Style::default().fg(theme::TEXT_DIM),
            )));
        }
    }
    let widget = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}

fn draw_resources_tab(f: &mut Frame, app: &App, proc: &sysinfo::Process, area: Rect) {
    let cpu_count = app.process_manager.cpu_count() as f32;
    let cpu_normalized = proc.cpu_usage() / cpu_count;
    let memory_mb = proc.memory() as f64 / 1_048_576.0;

    let runtime_str = format_duration(proc.run_time());
    let cpu_bar = theme::gradient_bar(cpu_normalized, 20, theme::cpu_color(cpu_normalized));

    let lines = vec![
        section_header("CPU"),
        Line::from(vec![
            Span::styled("  Usage: [", Style::default().fg(theme::TEXT_LABEL)),
            cpu_bar.spans[0].clone(),
            Span::styled("] ", Style::default().fg(theme::TEXT_LABEL)),
            Span::styled(
                format!("{:.1}%", cpu_normalized),
                Style::default()
                    .fg(theme::cpu_color(cpu_normalized))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        info_line("Runtime", runtime_str),
        Line::from(""),
        section_header("Memory"),
        info_line("RSS", format!("{:.1} MB", memory_mb)),
        info_line(
            "Virtual",
            format!("{:.1} MB", proc.virtual_memory() as f64 / 1_048_576.0),
        ),
        Line::from(""),
        section_header("Threads"),
        info_line(
            "Count",
            proc.tasks().map(|t| t.len()).unwrap_or(1).to_string(),
        ),
    ];

    let widget = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}

fn draw_io_tab(f: &mut Frame, proc: &sysinfo::Process, area: Rect) {
    let disk = proc.disk_usage();
    let lines = vec![
        section_header("Disk I/O"),
        info_line("Read Rate", format_bytes(disk.read_bytes)),
        info_line("Write Rate", format_bytes(disk.written_bytes)),
        info_line("Total Read", format_bytes(disk.total_read_bytes)),
        info_line("Total Written", format_bytes(disk.total_written_bytes)),
    ];
    let widget = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}

fn draw_connections_tab(f: &mut Frame, proc: &sysinfo::Process, area: Rect) {
    let pid = proc.pid().as_u32();
    let mut lines = vec![section_header("Active Network Connections")];

    if let Ok(output) = Command::new("lsof")
        .args(["-nP", "-i", "-p", &pid.to_string()])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let output_lines: Vec<_> = stdout.lines().skip(1).collect();

        if output_lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No active IP connections found.",
                Style::default().fg(theme::TEXT_DIM),
            )));
        } else {
            for l in output_lines.iter().take(15) {
                let parts: Vec<_> = l.split_whitespace().collect();
                if parts.len() >= 9 {
                    let proto = parts[7].to_string();
                    let addr = parts[8].to_string();
                    let state = parts.get(9).unwrap_or(&"").to_string();
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {:<4} ", proto),
                            Style::default().fg(theme::TEXT_ACCENT),
                        ),
                        Span::styled(
                            format!("{:<30}", addr),
                            Style::default().fg(theme::TEXT_VALUE),
                        ),
                        Span::styled(state, Style::default().fg(theme::TEXT_DIM)),
                    ]));
                }
            }
            if output_lines.len() > 15 {
                lines.push(Line::from(Span::styled(
                    format!("  ... and {} more", output_lines.len() - 15),
                    Style::default().fg(theme::TEXT_DIM),
                )));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  lsof not available or permission denied.",
            Style::default().fg(theme::TEXT_DIM),
        )));
    }

    let widget = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}

fn draw_relations_tab(
    f: &mut Frame,
    _app: &App,
    proc: &sysinfo::Process,
    processes: &[&sysinfo::Process],
    area: Rect,
) {
    let pid = proc.pid().as_u32();
    let mut lines = vec![section_header("Relations")];

    let children: Vec<_> = processes
        .iter()
        .filter(|p| p.parent().map(|pp| pp.as_u32()) == Some(pid))
        .collect();
    lines.push(info_line("Children Count", children.len().to_string()));
    for c in children.iter().take(5) {
        lines.push(Line::from(vec![
            Span::styled("    └─ ", Style::default().fg(theme::TEXT_DIM)),
            Span::styled(
                c.pid().as_u32().to_string(),
                Style::default().fg(theme::TEXT_ACCENT),
            ),
            Span::styled(
                format!(" {}", c.name()),
                Style::default().fg(theme::TEXT_VALUE),
            ),
        ]));
    }

    let widget = Paragraph::new(lines).wrap(Wrap { trim: true });
    f.render_widget(widget, area);
}

fn draw_signal_menu(f: &mut Frame, app: &App, proc: &sysinfo::Process, area: Rect) {
    let block = Block::default()
        .title(Span::styled(
            format!(" Send signal to PID {} ({}) ", proc.pid(), proc.name()),
            Style::default()
                .fg(theme::BORDER_DETAIL)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER_DETAIL));

    f.render_widget(block, area);
    let inner = area.inner(ratatui::layout::Margin {
        horizontal: 1,
        vertical: 1,
    });

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Prompt
            Constraint::Min(0),    // Grid
            Constraint::Length(5), // Footer
        ])
        .split(inner);

    // Prompt
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                "  Enter signal number: ",
                Style::default().fg(theme::TEXT_LABEL),
            ),
            Span::styled(
                &app.signal_input,
                Style::default()
                    .fg(theme::TEXT_VALUE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" █", Style::default().fg(theme::TEXT_ACCENT)),
        ])),
        chunks[0],
    );

    // Grid (4 columns, 8 rows)
    let signals = [
        "SIGHUP",
        "SIGINT",
        "SIGQUIT",
        "SIGILL",
        "SIGTRAP",
        "SIGABRT",
        "SIGEMT",
        "SIGFPE",
        "SIGKILL",
        "SIGBUS",
        "SIGSEGV",
        "SIGSYS",
        "SIGPIPE",
        "SIGALRM",
        "SIGTERM",
        "SIGURG",
        "SIGSTOP",
        "SIGTSTP",
        "SIGCONT",
        "SIGCHLD",
        "SIGTTIN",
        "SIGTTOU",
        "SIGIO",
        "SIGXCPU",
        "SIGXFSZ",
        "SIGVTALRM",
        "SIGPROF",
        "SIGWINCH",
        "SIGINFO",
        "SIGUSR1",
        "SIGUSR2",
    ];

    let mut signal_lines = Vec::new();
    for r in 0..8 {
        let mut spans = Vec::new();
        for c in 0..4 {
            let idx = r * 4 + c;
            if idx < signals.len() {
                let sig_num = idx + 1;
                let sig_name = signals[idx];
                let is_selected = app.signal_index == idx;

                let style = if is_selected {
                    Style::default()
                        .bg(theme::PROC_SELECTED_BG)
                        .fg(theme::PROC_SELECTED_FG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_VALUE)
                };

                spans.push(Span::styled(
                    format!("{:<2} ", sig_num),
                    style.fg(theme::cpu_color(sig_num as f32 * 3.0)),
                ));
                spans.push(Span::styled(format!("({:<10})  ", sig_name), style));
            }
        }
        signal_lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(signal_lines), chunks[1]);

    // Footer
    let footer = vec![
        Line::from(vec![
            Span::styled("  ↑ ↓ ← → ", Style::default().fg(theme::cpu_color(100.0))),
            Span::styled(" │ To choose signal.", Style::default().fg(theme::TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled("      0-9 ", Style::default().fg(theme::cpu_color(100.0))),
            Span::styled(" │ Enter manually.", Style::default().fg(theme::TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled("    ENTER ", Style::default().fg(theme::cpu_color(100.0))),
            Span::styled(" │ To send signal.", Style::default().fg(theme::TEXT_DIM)),
        ]),
        Line::from(vec![
            Span::styled(
                " ESC or \"q\" ",
                Style::default().fg(theme::cpu_color(100.0)),
            ),
            Span::styled(" │ To abort.", Style::default().fg(theme::TEXT_DIM)),
        ]),
    ];
    f.render_widget(Paragraph::new(footer), chunks[2]);
}

// ── Helpers ──────────────────────────────────────────────
fn info_line<'a>(label: impl Into<String>, value: impl Into<String>) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("  {:<12}: ", label.into()),
            Style::default().fg(theme::TEXT_LABEL),
        ),
        Span::styled(value.into(), Style::default().fg(theme::TEXT_VALUE)),
    ])
}

fn section_header<'a>(title: &'a str) -> Line<'a> {
    Line::from(Span::styled(
        format!(" ━━━ {} ━━━━━━━━━━━━━━━━━━━━━━━━", title),
        Style::default()
            .fg(theme::TEXT_SECTION)
            .add_modifier(Modifier::BOLD),
    ))
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

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{}h {:02}m {:02}s", h, m, s)
}

fn format_unix_time(seconds: u64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    let d = UNIX_EPOCH + Duration::from_secs(seconds);
    format!("{:?}", d)
}
