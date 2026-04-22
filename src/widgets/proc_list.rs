use crate::app::{App, AppMode};
use crate::sort::SortBy;
use crate::theme;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Cell, Row, Table},
    Frame,
};
use std::collections::{HashMap, HashSet};

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let processes = app
        .process_manager
        .get_filtered_and_sorted_processes(&app.search_query, app.sort_by);
    let cpu_count = app.process_manager.cpu_count() as f32;
    let total_mem = app.process_manager.total_memory();

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

    let mut rows = Vec::new();

    if app.tree_mode {
        let tree_lines = build_tree_view(app, &processes, cpu_count, total_mem);
        if let Some(selected_pid) = app.selected_pid {
            if let Some(pos) = tree_lines.iter().position(|line| line.pid == selected_pid) {
                app.selected_index = pos;
                app.table_state.select(Some(pos));
            }
        } else if !tree_lines.is_empty() {
            app.selected_index = 0;
            app.table_state.select(Some(0));
        }

        for line in tree_lines {
            let watch_symbol = if app.is_watchlisted(line.pid) {
                "★"
            } else {
                " "
            };
            let alerting = app.is_pid_alerting(line.pid);
            let row_style = if alerting {
                line.style
                    .bg(theme::STATUS_ZOMBIE)
                    .fg(theme::PROC_SELECTED_FG)
                    .add_modifier(Modifier::BOLD)
            } else {
                line.style
            };

            let cells = vec![
                Cell::from(format!("{}{}", watch_symbol, line.pid)),
                Cell::from(line.name),
                Cell::from("-".to_string()),
                Cell::from(format!("{:.1}", line.cpu))
                    .style(Style::default().fg(theme::cpu_color(line.cpu))),
                Cell::from(format!("{:.1}", line.mem_pct))
                    .style(Style::default().fg(theme::mem_color(line.mem_pct as f32))),
                Cell::from(format!("{:.1}", line.mem_mb)),
                Cell::from(line.threads.to_string()),
                Cell::from(line.state).style(Style::default().fg(line.state_color)),
                Cell::from(format!("{:.0}K", line.read_kb)),
                Cell::from(format!("{:.0}K", line.write_kb)),
            ];
            rows.push(Row::new(cells).style(row_style).height(1));
        }
    } else {
        for p in processes.iter() {
            let pid = p.pid().as_u32();
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

            let user_str = p
                .user_id()
                .and_then(|uid| {
                    app.process_manager
                        .users
                        .get_user_by_id(uid)
                        .map(|u| u.name().to_string())
                })
                .unwrap_or_else(|| {
                    p.user_id()
                        .map(|u| u.to_string())
                        .unwrap_or_else(|| "-".to_string())
                });

            let threads = p.tasks().map(|t| t.len()).unwrap_or(1);
            let mut row_style =
                Style::default().fg(theme::process_row_color(cpu_normalized, mem_mb));
            if app.is_pid_alerting(pid) {
                row_style = row_style
                    .bg(theme::STATUS_ZOMBIE)
                    .fg(theme::PROC_SELECTED_FG)
                    .add_modifier(Modifier::BOLD);
            }

            let watch_symbol = if app.is_watchlisted(pid) { "★" } else { " " };
            let cells = vec![
                Cell::from(format!("{}{}", watch_symbol, pid)),
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
            rows.push(Row::new(cells).style(row_style).height(1));
        }
    }

    let lock_indicator = if app.is_locked { " [L]" } else { "" };
    let tree_indicator = if app.tree_mode { " [T]" } else { "" };
    let watch_indicator = format!(" [W:{}]", app.watchlist_count());
    let alert_indicator = if app.alert_rules().enabled {
        format!(" [A:{}]", app.alerts_active_count())
    } else {
        " [A:off]".to_string()
    };
    let sort_name = format!("{:?}", app.sort_by);

    let title = if app.mode == AppMode::Filter {
        format!(
            " Processes (filter: {}_) [sort:{}]{}{}{}{} ",
            app.search_query,
            sort_name,
            lock_indicator,
            tree_indicator,
            watch_indicator,
            alert_indicator
        )
    } else if app.tree_mode {
        format!(
            " Processes [f:filter j/k:move ←/→:collapse/expand space:toggle w:watch a:alerts q:quit]{}{}{}{} ",
            lock_indicator, tree_indicator, watch_indicator, alert_indicator
        )
    } else {
        format!(
            " Processes [f:filter j/k:move s:sort w:watch a:alerts v:kill Enter:details t:tree q:quit]{}{}{}{} ",
            lock_indicator, tree_indicator, watch_indicator, alert_indicator
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

#[derive(Clone, Copy, Default)]
struct Aggregate {
    cpu: f32,
    mem: u64,
    threads: usize,
    read: u64,
    write: u64,
}

struct TreeLine {
    pid: u32,
    name: String,
    cpu: f32,
    mem_pct: f64,
    mem_mb: f32,
    threads: usize,
    state: String,
    state_color: Color,
    read_kb: f64,
    write_kb: f64,
    style: Style,
}

fn build_tree_view(
    app: &App,
    processes: &[&sysinfo::Process],
    cpu_count: f32,
    total_mem: u64,
) -> Vec<TreeLine> {
    let pid_set: HashSet<u32> = processes.iter().map(|p| p.pid().as_u32()).collect();
    let mut children_map = build_children_map(processes);
    let mut proc_map: HashMap<u32, &&sysinfo::Process> = HashMap::new();
    for process in processes {
        proc_map.insert(process.pid().as_u32(), process);
    }

    let mut roots: Vec<u32> = processes
        .iter()
        .filter(|p| {
            p.parent()
                .map(|pp| !pid_set.contains(&pp.as_u32()))
                .unwrap_or(true)
        })
        .map(|p| p.pid().as_u32())
        .collect();
    roots.sort_unstable();

    let mut aggregate_cache: HashMap<u32, Aggregate> = HashMap::new();

    fn aggregate(
        pid: u32,
        proc_map: &HashMap<u32, &&sysinfo::Process>,
        children_map: &HashMap<u32, Vec<u32>>,
        cpu_count: f32,
        cache: &mut HashMap<u32, Aggregate>,
    ) -> Aggregate {
        if let Some(cached) = cache.get(&pid).copied() {
            return cached;
        }
        let Some(process) = proc_map.get(&pid) else {
            return Aggregate::default();
        };

        let disk = process.disk_usage();
        let mut out = Aggregate {
            cpu: process.cpu_usage() / cpu_count,
            mem: process.memory(),
            threads: process.tasks().map(|t| t.len()).unwrap_or(1),
            read: disk.read_bytes,
            write: disk.written_bytes,
        };

        if let Some(children) = children_map.get(&pid) {
            for child in children {
                let child_total = aggregate(*child, proc_map, children_map, cpu_count, cache);
                out.cpu += child_total.cpu;
                out.mem += child_total.mem;
                out.threads += child_total.threads;
                out.read += child_total.read;
                out.write += child_total.write;
            }
        }

        cache.insert(pid, out);
        out
    }

    let mut lines = Vec::new();

    fn walk(
        app: &App,
        pid: u32,
        prefix: &str,
        is_last: bool,
        depth: usize,
        cpu_count: f32,
        total_mem: u64,
        proc_map: &HashMap<u32, &&sysinfo::Process>,
        children_map: &mut HashMap<u32, Vec<u32>>,
        cache: &mut HashMap<u32, Aggregate>,
        lines: &mut Vec<TreeLine>,
    ) {
        if depth > 16 || lines.len() > 600 {
            return;
        }
        let Some(process) = proc_map.get(&pid) else {
            return;
        };

        let mut children = children_map.get(&pid).cloned().unwrap_or_default();
        for child in &children {
            let _ = aggregate(*child, proc_map, children_map, cpu_count, cache);
        }
        children.sort_by(|a, b| {
            let a_cpu = cache.get(a).map(|c| c.cpu).unwrap_or(0.0);
            let b_cpu = cache.get(b).map(|c| c.cpu).unwrap_or(0.0);
            b_cpu
                .partial_cmp(&a_cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if !children.is_empty() {
            children_map.insert(pid, children.clone());
        }

        let has_children = !children.is_empty();
        let expanded = has_children && !app.collapsed_tree_nodes.contains(&pid);

        let connector = if depth == 0 {
            String::new()
        } else if is_last {
            format!("{}└─", prefix)
        } else {
            format!("{}├─", prefix)
        };
        let indicator = if has_children {
            if expanded {
                "▾"
            } else {
                "▸"
            }
        } else {
            "•"
        };

        let totals = aggregate(pid, proc_map, children_map, cpu_count, cache);
        let mem_pct = if total_mem > 0 {
            (totals.mem as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        };
        let mem_mb = totals.mem as f32 / 1_048_576.0;

        let state_color = match process.status() {
            sysinfo::ProcessStatus::Run => theme::STATUS_RUN,
            sysinfo::ProcessStatus::Zombie => theme::STATUS_ZOMBIE,
            sysinfo::ProcessStatus::Stop => theme::STATUS_STOP,
            _ => theme::STATUS_SLEEP,
        };

        lines.push(TreeLine {
            pid,
            name: format!("{}{} {}", connector, indicator, process.name()),
            cpu: totals.cpu,
            mem_pct,
            mem_mb,
            threads: totals.threads,
            state: format!("{:?}", process.status()),
            state_color,
            read_kb: totals.read as f64 / 1024.0,
            write_kb: totals.write as f64 / 1024.0,
            style: Style::default().fg(theme::process_row_color(totals.cpu, mem_mb)),
        });

        if !expanded {
            return;
        }

        let child_prefix = if depth == 0 {
            String::new()
        } else if is_last {
            format!("{}  ", prefix)
        } else {
            format!("{}│ ", prefix)
        };

        for (idx, child_pid) in children.iter().enumerate() {
            walk(
                app,
                *child_pid,
                &child_prefix,
                idx == children.len() - 1,
                depth + 1,
                cpu_count,
                total_mem,
                proc_map,
                children_map,
                cache,
                lines,
            );
        }
    }

    for (idx, root_pid) in roots.iter().enumerate() {
        walk(
            app,
            *root_pid,
            "",
            idx == roots.len() - 1,
            0,
            cpu_count,
            total_mem,
            &proc_map,
            &mut children_map,
            &mut aggregate_cache,
            &mut lines,
        );
    }

    lines
}

fn build_children_map(processes: &[&sysinfo::Process]) -> HashMap<u32, Vec<u32>> {
    let mut children: HashMap<u32, Vec<u32>> = HashMap::new();
    for process in processes {
        if let Some(parent) = process.parent() {
            children
                .entry(parent.as_u32())
                .or_default()
                .push(process.pid().as_u32());
        }
    }
    children
}
