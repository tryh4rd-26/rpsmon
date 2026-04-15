use crate::app::{App, AppMode};
use crate::sort::SortBy;
use crate::theme;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let processes = app
        .process_manager
        .get_filtered_and_sorted_processes(&app.search_query, app.sort_by);
    let cpu_count = app.process_manager.cpu_count() as f32;

    // Build header with sort indicator
    let sort_indicator = |col: SortBy| -> &str {
        if app.sort_by == col {
            " ▼"
        } else {
            ""
        }
    };

    let headers = vec![
        format!("PID{}", sort_indicator(SortBy::Pid)),
        format!("NAME{}", sort_indicator(SortBy::Name)),
        "USER".to_string(),
        format!("CPU%{}", sort_indicator(SortBy::Cpu)),
        format!("MEM%{}", sort_indicator(SortBy::Memory)),
        "MEM MB".to_string(),
        format!("THR{}", sort_indicator(SortBy::Threads)),
        "STATE".to_string(),
        "READ".to_string(),
        "WRITE".to_string(),
    ];

    let header_cells = headers.iter().map(|h| {
        Cell::from(h.as_str()).style(
            Style::default()
                .fg(theme::TEXT_SECTION)
                .add_modifier(Modifier::BOLD),
        )
    });

    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let total_mem = app.process_manager.total_memory();

    // Build rows
    let mut rows = Vec::new();

    if app.tree_mode {
        // Tree view: build hierarchical process list
        let tree_lines = build_tree_view(&processes, cpu_count, total_mem);
        for (line_cells, row_style) in tree_lines {
            rows.push(Row::new(line_cells).style(row_style).height(1));
        }
    } else {
        for p in processes.iter() {
            let cpu_normalized = p.cpu_usage() / cpu_count;
            let mem_mb = p.memory() as f32 / 1_048_576.0;
            let mem_pct = if total_mem > 0 {
                (p.memory() as f64 / total_mem as f64) * 100.0
            } else {
                0.0
            };

            let disk = p.disk_usage();
            let read_kb = disk.read_bytes as f64 / 1024.0;
            let write_kb = disk.written_bytes as f64 / 1024.0;

            let state_str = format!("{:?}", p.status());
            let state_color = match p.status() {
                sysinfo::ProcessStatus::Run => theme::STATUS_RUN,
                sysinfo::ProcessStatus::Zombie => theme::STATUS_ZOMBIE,
                sysinfo::ProcessStatus::Stop => theme::STATUS_STOP,
                _ => theme::STATUS_SLEEP,
            };

            let user_str = p.user_id().and_then(|uid| {
                app.process_manager.users.get_user_by_id(uid).map(|u| u.name().to_string())
            }).unwrap_or_else(|| p.user_id().map(|u| u.to_string()).unwrap_or_else(|| "-".to_string()));

            let threads = p.tasks().map(|t| t.len()).unwrap_or(1);

            let row_color = theme::process_row_color(cpu_normalized, mem_mb);

            let cells = vec![
                Cell::from(p.pid().as_u32().to_string()),
                Cell::from(p.name().to_string()),
                Cell::from(user_str),
                Cell::from(format!("{:.1}", cpu_normalized))
                    .style(Style::default().fg(theme::cpu_color(cpu_normalized))),
                Cell::from(format!("{:.1}", mem_pct))
                    .style(Style::default().fg(theme::mem_color(mem_pct as f32))),
                Cell::from(format!("{:.1}", mem_mb)),
                Cell::from(threads.to_string()),
                Cell::from(state_str).style(Style::default().fg(state_color)),
                Cell::from(format!("{:.0}K", read_kb)),
                Cell::from(format!("{:.0}K", write_kb)),
            ];

            rows.push(
                Row::new(cells)
                    .style(Style::default().fg(row_color))
                    .height(1),
            );
        }
    }

    // Build status line for title
    let lock_indicator = if app.is_locked { " [L]" } else { "" };
    let tree_indicator = if app.tree_mode { " [T]" } else { "" };
    let sort_name = format!("{:?}", app.sort_by);

    let title = if app.mode == AppMode::Filter {
        format!(
            " Processes (filter: {}_) [sort:{}]{}{} ",
            app.search_query, sort_name, lock_indicator, tree_indicator
        )
    } else {
        format!(
            " Processes [f:filter j/k:up/down s:sort v:kill Enter:details t:tree l:lock q:quit]{}{} ",
            lock_indicator, tree_indicator
        )
    };

    let widths = [
        Constraint::Length(7),
        Constraint::Percentage(18),
        Constraint::Length(8),
        Constraint::Length(6),
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Length(4),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(8),
    ];

    let border_color = if app.mode == AppMode::Filter {
        theme::BORDER_CPU
    } else {
        theme::BORDER_PROC
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(Span::styled(
                    title,
                    Style::default()
                        .fg(border_color)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(
            Style::default()
                .bg(theme::PROC_SELECTED_BG)
                .fg(theme::PROC_SELECTED_FG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn build_tree_view<'a>(
    processes: &[&sysinfo::Process],
    cpu_count: f32,
    total_mem: u64,
) -> Vec<(Vec<Cell<'a>>, Style)> {
    use std::collections::HashMap;

    // Build parent → children map
    let pid_set: std::collections::HashSet<u32> =
        processes.iter().map(|p| p.pid().as_u32()).collect();

    let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();
    let mut proc_map: HashMap<u32, &&sysinfo::Process> = HashMap::new();

    for p in processes {
        let pid = p.pid().as_u32();
        proc_map.insert(pid, p);
        if let Some(parent) = p.parent() {
            let ppid = parent.as_u32();
            children_map.entry(ppid).or_default().push(pid);
        }
    }

    // Find root processes (no parent in our set)
    let mut roots: Vec<u32> = processes
        .iter()
        .filter(|p| {
            p.parent()
                .map(|pp| !pid_set.contains(&pp.as_u32()))
                .unwrap_or(true)
        })
        .map(|p| p.pid().as_u32())
        .collect();
    roots.sort();
    roots.truncate(200); // Limit to prevent slowdown

    let mut result = Vec::new();

    fn walk<'a>(
        pid: u32,
        prefix: &str,
        is_last: bool,
        proc_map: &HashMap<u32, &&sysinfo::Process>,
        children_map: &HashMap<u32, Vec<u32>>,
        cpu_count: f32,
        total_mem: u64,
        result: &mut Vec<(Vec<Cell<'a>>, Style)>,
        depth: usize,
    ) {
        if depth > 8 {
            return;
        }
        let Some(p) = proc_map.get(&pid) else {
            return;
        };

        let connector = if depth == 0 {
            "".to_string()
        } else if is_last {
            format!("{}└─", prefix)
        } else {
            format!("{}├─", prefix)
        };

        let cpu_normalized = p.cpu_usage() / cpu_count;
        let mem_mb = p.memory() as f32 / 1_048_576.0;
        let mem_pct = if total_mem > 0 {
            (p.memory() as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        };
        let threads = p.tasks().map(|t| t.len()).unwrap_or(1);
        let state_str = format!("{:?}", p.status());
        let disk = p.disk_usage();

        let row_color = theme::process_row_color(cpu_normalized, mem_mb);

        let cells = vec![
            Cell::from(pid.to_string()),
            Cell::from(format!("{}{}", connector, p.name())),
            Cell::from("-".to_string()),
            Cell::from(format!("{:.1}", cpu_normalized)),
            Cell::from(format!("{:.1}", mem_pct)),
            Cell::from(format!("{:.1}", mem_mb)),
            Cell::from(threads.to_string()),
            Cell::from(state_str),
            Cell::from(format!("{:.0}K", disk.read_bytes as f64 / 1024.0)),
            Cell::from(format!("{:.0}K", disk.written_bytes as f64 / 1024.0)),
        ];

        result.push((cells, Style::default().fg(row_color)));

        if let Some(children) = children_map.get(&pid) {
            let child_prefix = if depth == 0 {
                "".to_string()
            } else if is_last {
                format!("{}  ", prefix)
            } else {
                format!("{}│ ", prefix)
            };

            for (i, &child_pid) in children.iter().enumerate() {
                let child_is_last = i == children.len() - 1;
                walk(
                    child_pid,
                    &child_prefix,
                    child_is_last,
                    proc_map,
                    children_map,
                    cpu_count,
                    total_mem,
                    result,
                    depth + 1,
                );
            }
        }
    }

    for (i, &root_pid) in roots.iter().enumerate() {
        let is_last = i == roots.len() - 1;
        walk(
            root_pid,
            "",
            is_last,
            &proc_map,
            &children_map,
            cpu_count,
            total_mem,
            &mut result,
            0,
        );
    }

    result
}
