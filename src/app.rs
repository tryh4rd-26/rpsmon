use crate::config::Config;
use crate::events::{Event, EventHandler};
use crate::keys::KeyHandler;
use crate::process::ProcessManager;
use crate::sort::SortBy;
use crate::ui;
use anyhow::Result;
use ratatui::backend::Backend;
use ratatui::widgets::TableState;
use ratatui::Terminal;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppMode {
    Normal,
    Filter,
    Detail,
    SignalMenu,
}

#[derive(Debug, Clone)]
pub struct AlertRules {
    pub enabled: bool,
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub hold_secs: u64,
}

#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub pid: u32,
    pub process_name: String,
    pub metric: &'static str,
    pub value: f32,
    pub threshold: f32,
    pub epoch_secs: u64,
}

#[derive(Debug, Default, Clone, Copy)]
struct AlertBreachState {
    cpu_ticks: u64,
    mem_ticks: u64,
    cpu_active: bool,
    mem_active: bool,
}

#[derive(Debug, Clone)]
struct AlertSample {
    pid: u32,
    process_name: String,
    cpu_pct: f32,
    mem_pct: f32,
}

pub struct App {
    pub should_quit: bool,
    pub process_manager: ProcessManager,
    pub config: Config,
    pub selected_index: usize,
    pub table_state: TableState,
    pub search_query: String,
    pub mode: AppMode,
    pub sort_by: SortBy,
    pub cpu_history: VecDeque<f32>, // CPU usage history for graph (60 pts)
    pub mem_history: VecDeque<f32>, // Memory usage history for graph (60 pts)
    pub net_history: VecDeque<(f64, f64)>, // (rx_rate, tx_rate) in bytes/sec (60 pts)
    pub disk_io_history: VecDeque<(u64, u64)>,
    pub per_disk_history: HashMap<String, VecDeque<u64>>,
    pub signal_input: String,
    pub signal_index: usize, // Selection in the 1-31 signal grid
    pub selected_pid: Option<u32>,
    pub is_locked: bool,
    pub show_details: bool,
    pub tree_mode: bool,
    pub watchlist: HashSet<u32>,
    pub collapsed_tree_nodes: HashSet<u32>,
    tree_visible_pids: Vec<u32>,
    alert_rules: AlertRules,
    active_alert_pids: HashSet<u32>,
    alert_state: HashMap<u32, AlertBreachState>,
    alert_log: VecDeque<AlertEvent>,
    pub detail_tab: usize, // 0=Identity, 1=Resources, 2=IO, 3=Conn, 4=Relations, 5=Env
    pub last_key_time: std::time::Instant,
    pub last_key_was_d: bool,
    // Rate calculation state
    prev_net: (u64, u64),
    prev_disk_read: u64,
    prev_disk_write: u64,
    current_net_rate: (f64, f64),
    current_disk_rate: (f64, f64),
}

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let sort_by = SortBy::from_str(&config.sort_by);
        let watchlist = config.watchlist_pids.iter().copied().collect();
        let alert_rules = AlertRules {
            enabled: config.alerts_enabled,
            cpu_pct: config.alert_cpu_pct.clamp(1.0, 100.0),
            mem_pct: config.alert_mem_pct.clamp(0.1, 100.0),
            hold_secs: config.alert_hold_secs.max(1),
        };
        let process_manager = ProcessManager::new();

        let mut table_state = TableState::default();
        table_state.select(Some(0));

        let (rx, tx) = process_manager.get_network_stats();

        Ok(Self {
            should_quit: false,
            process_manager,
            config,
            selected_index: 0,
            table_state,
            search_query: String::new(),
            mode: AppMode::Normal,
            sort_by,
            cpu_history: VecDeque::with_capacity(60),
            mem_history: VecDeque::with_capacity(60),
            net_history: VecDeque::with_capacity(60),
            disk_io_history: VecDeque::with_capacity(60),
            per_disk_history: HashMap::new(),
            selected_pid: None,
            is_locked: false,
            show_details: false,
            tree_mode: false,
            watchlist,
            collapsed_tree_nodes: HashSet::new(),
            tree_visible_pids: Vec::new(),
            alert_rules,
            active_alert_pids: HashSet::new(),
            alert_state: HashMap::new(),
            alert_log: VecDeque::with_capacity(64),
            detail_tab: 0,
            signal_input: String::new(),
            signal_index: 14, // Default to SIGTERM (15) which is index 14
            last_key_time: std::time::Instant::now(),
            last_key_was_d: false,
            prev_net: (rx, tx),
            prev_disk_read: 0,
            prev_disk_write: 0,
            current_net_rate: (0.0, 0.0),
            current_disk_rate: (0.0, 0.0),
        })
    }

    pub fn update_data(&mut self) {
        self.process_manager.refresh();

        // ── Process selection tracking ──
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        let visible_pid_set: HashSet<u32> = processes.iter().map(|p| p.pid().as_u32()).collect();
        self.collapsed_tree_nodes
            .retain(|pid| visible_pid_set.contains(pid));
        if self.tree_mode {
            self.tree_visible_pids =
                compute_tree_visible_pids(&processes, &self.collapsed_tree_nodes);
        } else {
            self.tree_visible_pids.clear();
        }
        let nav_pids = if self.tree_mode {
            self.tree_visible_pids.clone()
        } else {
            processes.iter().map(|p| p.pid().as_u32()).collect()
        };
        self.refresh_selection_from_navigation(&nav_pids);
        let alert_cpu_count = self.process_manager.cpu_count().max(1) as f32;
        let alert_total_mem = self.process_manager.total_memory().max(1) as f64;
        let alert_samples: Vec<AlertSample> = self
            .process_manager
            .get_all_processes()
            .into_iter()
            .map(|process| AlertSample {
                pid: process.pid().as_u32(),
                process_name: process.name().to_string(),
                cpu_pct: process.cpu_usage() / alert_cpu_count,
                mem_pct: (process.memory() as f64 / alert_total_mem * 100.0) as f32,
            })
            .collect();
        self.update_alerts(&alert_samples);

        // ── CPU + Memory history ──
        let total = self.process_manager.total_memory();
        let used = self.process_manager.used_memory();
        let mem_ratio = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        push_capped(
            &mut self.cpu_history,
            self.process_manager.global_cpu_usage(),
            60,
        );
        push_capped(&mut self.mem_history, mem_ratio, 60);

        // ── Network rate history ──
        let (rx_now, tx_now) = self.process_manager.get_network_stats();
        let rx_rate = rx_now.saturating_sub(self.prev_net.0) as f64;
        let tx_rate = tx_now.saturating_sub(self.prev_net.1) as f64;
        self.prev_net = (rx_now, tx_now);
        self.current_net_rate = (rx_rate, tx_rate);

        push_capped(&mut self.net_history, (rx_rate, tx_rate), 60);

        // ── Disk I/O rate history ──
        let (disk_read, disk_write) = self.compute_total_disk_io();
        let read_rate = disk_read.saturating_sub(self.prev_disk_read);
        let write_rate = disk_write.saturating_sub(self.prev_disk_write);
        self.prev_disk_read = disk_read;
        self.prev_disk_write = disk_write;
        self.current_disk_rate = (read_rate as f64, write_rate as f64);

        push_capped(&mut self.disk_io_history, (read_rate, write_rate), 60);

        // Track per-disk sparklines
        for disk in self.process_manager.get_disks() {
            let name = disk.filesystem.clone();
            let history = self
                .per_disk_history
                .entry(name)
                .or_insert_with(|| VecDeque::with_capacity(60));
            push_capped(history, read_rate + write_rate, 60);
        }

        if self.show_details && self.detail_tab == 3 {
            if let Some(pid) = self.selected_pid {
                self.process_manager.refresh_process_connections(pid);
            }
        }

        // Reset double-tap detection if timeout exceeded
        if self.last_key_time.elapsed() > Duration::from_secs(1) {
            self.last_key_was_d = false;
        }
    }

    fn compute_total_disk_io(&self) -> (u64, u64) {
        let mut total_read = 0u64;
        let mut total_write = 0u64;
        for p in self.process_manager.get_all_processes() {
            let d = p.disk_usage();
            total_read += d.total_read_bytes;
            total_write += d.total_written_bytes;
        }
        (total_read, total_write)
    }

    /// Get current network rate (bytes/sec)
    pub fn net_rate(&self) -> (f64, f64) {
        self.current_net_rate
    }

    /// Get current disk I/O rate (bytes/sec)
    pub fn disk_io_rate(&self) -> (f64, f64) {
        self.current_disk_rate
    }

    pub fn handle_key(&mut self, key_event: crossterm::event::KeyEvent) {
        KeyHandler::handle(self, key_event);
    }

    pub fn select_next(&mut self) {
        let nav_pids = self.current_navigation_pids();
        if nav_pids.is_empty() {
            return;
        }
        let current_pos = self.current_navigation_index(&nav_pids);
        if current_pos < nav_pids.len() - 1 {
            self.select_navigation_index(current_pos + 1, &nav_pids);
        }
    }

    pub fn select_previous(&mut self) {
        let nav_pids = self.current_navigation_pids();
        if nav_pids.is_empty() {
            return;
        }
        let current_pos = self.current_navigation_index(&nav_pids);
        if current_pos > 0 {
            self.select_navigation_index(current_pos - 1, &nav_pids);
        }
    }

    pub fn select_top(&mut self) {
        let nav_pids = self.current_navigation_pids();
        if !nav_pids.is_empty() {
            self.select_navigation_index(0, &nav_pids);
        }
    }

    pub fn select_bottom(&mut self) {
        let nav_pids = self.current_navigation_pids();
        if !nav_pids.is_empty() {
            self.select_navigation_index(nav_pids.len() - 1, &nav_pids);
        }
    }

    pub fn send_custom_signal(&mut self, signal: i32) -> Result<()> {
        if let Some(pid) = self.selected_pid {
            self.process_manager
                .send_signal(sysinfo::Pid::from_u32(pid), signal)?;
            Ok(())
        } else {
            anyhow::bail!("No process selected")
        }
    }

    pub fn page_down(&mut self) {
        let nav_pids = self.current_navigation_pids();
        if !nav_pids.is_empty() {
            let idx = (self.current_navigation_index(&nav_pids) + 20).min(nav_pids.len() - 1);
            self.select_navigation_index(idx, &nav_pids);
        }
    }

    pub fn page_up(&mut self) {
        let nav_pids = self.current_navigation_pids();
        if !nav_pids.is_empty() {
            let idx = self.current_navigation_index(&nav_pids).saturating_sub(20);
            self.select_navigation_index(idx, &nav_pids);
        }
    }

    pub fn add_search_char(&mut self, c: char) {
        self.search_query.push(c);
        self.selected_index = 0;
        self.table_state.select(Some(0));
    }

    pub fn pop_search_char(&mut self) {
        self.search_query.pop();
        self.selected_index = 0;
        self.table_state.select(Some(0));
    }

    pub fn kill_selected(&mut self) {
        if let Some(pid) = self.selected_pid {
            let _ = self
                .process_manager
                .kill_process(sysinfo::Pid::from_u32(pid));
        }
    }

    pub fn sigkill_selected(&mut self) {
        if let Some(pid) = self.selected_pid {
            let _ = self
                .process_manager
                .send_signal(sysinfo::Pid::from_u32(pid), libc::SIGKILL);
        }
    }

    pub fn toggle_pause_selected(&mut self) {
        if let Some(pid) = self.selected_pid {
            let process_map = self.process_manager.sys().processes();
            if let Some(process) = process_map.get(&sysinfo::Pid::from_u32(pid)) {
                let status = process.status();
                let sig = match status {
                    sysinfo::ProcessStatus::Stop => libc::SIGCONT,
                    _ => libc::SIGSTOP,
                };
                let _ = self
                    .process_manager
                    .send_signal(sysinfo::Pid::from_u32(pid), sig);
            }
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = self.sort_by.cycle();
        self.config.sort_by = self.sort_by.as_str().to_string();
        self.refresh_selection_from_navigation(&self.current_navigation_pids());
    }

    pub fn cycle_detail_tab(&mut self) {
        self.detail_tab = (self.detail_tab + 1) % 6;
    }

    pub fn toggle_tree_mode(&mut self) {
        self.tree_mode = !self.tree_mode;
        self.refresh_selection_from_navigation(&self.current_navigation_pids());
    }

    pub fn toggle_lock(&mut self) {
        self.is_locked = !self.is_locked;
    }

    pub fn toggle_watch_selected(&mut self) {
        if let Some(pid) = self.selected_pid {
            if !self.watchlist.insert(pid) {
                self.watchlist.remove(&pid);
            }
        }
    }

    pub fn is_watchlisted(&self, pid: u32) -> bool {
        self.watchlist.contains(&pid)
    }

    pub fn watchlist_count(&self) -> usize {
        self.watchlist.len()
    }

    pub fn watchlist_snapshot(&self, limit: usize) -> Vec<String> {
        let mut pids: Vec<u32> = self.watchlist.iter().copied().collect();
        pids.sort_unstable();
        let cpu_count = self.process_manager.cpu_count().max(1) as f32;
        let total_mem = self.process_manager.total_memory().max(1);
        let process_map = self.process_manager.sys().processes();
        let mut out = Vec::new();
        for pid in pids.into_iter().take(limit) {
            if let Some(proc_) = process_map.get(&sysinfo::Pid::from_u32(pid)) {
                let cpu = proc_.cpu_usage() / cpu_count;
                let mem_pct = (proc_.memory() as f64 / total_mem as f64) * 100.0;
                out.push(format!(
                    "{}:{} {:.1}%/{:.1}%",
                    pid,
                    proc_.name(),
                    cpu,
                    mem_pct
                ));
            } else {
                out.push(format!("{}:<exited>", pid));
            }
        }
        out
    }

    pub fn alert_rules(&self) -> &AlertRules {
        &self.alert_rules
    }

    pub fn alerts_active_count(&self) -> usize {
        self.active_alert_pids.len()
    }

    pub fn latest_alert(&self) -> Option<&AlertEvent> {
        self.alert_log.back()
    }

    pub fn toggle_alerts(&mut self) {
        self.alert_rules.enabled = !self.alert_rules.enabled;
        self.config.alerts_enabled = self.alert_rules.enabled;
        if !self.alert_rules.enabled {
            self.active_alert_pids.clear();
            for state in self.alert_state.values_mut() {
                state.cpu_active = false;
                state.mem_active = false;
            }
        }
    }

    pub fn toggle_tree_selected_expanded(&mut self) {
        if !self.tree_mode {
            return;
        }
        if let Some(pid) = self.selected_pid {
            if !self.tree_pid_has_children(pid) {
                return;
            }
            if !self.collapsed_tree_nodes.insert(pid) {
                self.collapsed_tree_nodes.remove(&pid);
            }
            self.tree_visible_pids = self.current_navigation_pids();
            self.refresh_selection_from_navigation(&self.tree_visible_pids.clone());
        }
    }

    pub fn expand_tree_selected(&mut self) {
        if !self.tree_mode {
            return;
        }
        if let Some(pid) = self.selected_pid {
            self.collapsed_tree_nodes.remove(&pid);
            self.tree_visible_pids = self.current_navigation_pids();
            self.refresh_selection_from_navigation(&self.tree_visible_pids.clone());
        }
    }

    pub fn collapse_tree_selected(&mut self) {
        if !self.tree_mode {
            return;
        }
        if let Some(pid) = self.selected_pid {
            if self.tree_pid_has_children(pid) {
                self.collapsed_tree_nodes.insert(pid);
            } else if let Some(parent) = self.tree_parent_of_selected() {
                self.selected_pid = Some(parent);
                self.collapsed_tree_nodes.insert(parent);
            }
            self.tree_visible_pids = self.current_navigation_pids();
            self.refresh_selection_from_navigation(&self.tree_visible_pids.clone());
        }
    }

    fn tree_pid_has_children(&self, pid: u32) -> bool {
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        let children = build_children_map(&processes);
        children.get(&pid).is_some_and(|childs| !childs.is_empty())
    }

    fn tree_parent_of_selected(&self) -> Option<u32> {
        let selected = self.selected_pid?;
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        let pids: HashSet<u32> = processes.iter().map(|p| p.pid().as_u32()).collect();
        let proc_map = self.process_manager.sys().processes();
        let proc_ = proc_map.get(&sysinfo::Pid::from_u32(selected))?;
        let parent = proc_.parent()?;
        if pids.contains(&parent.as_u32()) {
            Some(parent.as_u32())
        } else {
            None
        }
    }

    fn current_navigation_pids(&self) -> Vec<u32> {
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if self.tree_mode {
            compute_tree_visible_pids(&processes, &self.collapsed_tree_nodes)
        } else {
            processes.iter().map(|p| p.pid().as_u32()).collect()
        }
    }

    fn current_navigation_index(&self, nav_pids: &[u32]) -> usize {
        self.selected_pid
            .and_then(|pid| nav_pids.iter().position(|p| *p == pid))
            .unwrap_or_else(|| self.selected_index.min(nav_pids.len().saturating_sub(1)))
    }

    fn select_navigation_index(&mut self, index: usize, nav_pids: &[u32]) {
        if nav_pids.is_empty() {
            self.selected_index = 0;
            self.selected_pid = None;
            self.table_state.select(Some(0));
            return;
        }
        let idx = index.min(nav_pids.len() - 1);
        self.selected_index = idx;
        self.selected_pid = Some(nav_pids[idx]);
        self.table_state.select(Some(idx));
    }

    fn refresh_selection_from_navigation(&mut self, nav_pids: &[u32]) {
        if nav_pids.is_empty() {
            self.selected_index = 0;
            self.selected_pid = None;
            self.table_state.select(Some(0));
            return;
        }

        if self.is_locked && self.selected_pid.is_some_and(|pid| nav_pids.contains(&pid)) {
            let idx = self.current_navigation_index(nav_pids);
            self.select_navigation_index(idx, nav_pids);
            return;
        }

        let idx = self.current_navigation_index(nav_pids);
        self.select_navigation_index(idx, nav_pids);
    }

    fn update_alerts(&mut self, samples: &[AlertSample]) {
        if !self.alert_rules.enabled {
            self.active_alert_pids.clear();
            return;
        }

        let hold_ticks = self.alert_rules.hold_secs.max(1);

        let mut seen = HashSet::new();
        let mut active = HashSet::new();

        for sample in samples {
            let pid = sample.pid;
            seen.insert(pid);
            let cpu = sample.cpu_pct;
            let mem = sample.mem_pct;

            let mut state = self.alert_state.get(&pid).copied().unwrap_or_default();

            state.cpu_ticks = if cpu >= self.alert_rules.cpu_pct {
                state.cpu_ticks + 1
            } else {
                state.cpu_active = false;
                0
            };
            if !state.cpu_active && state.cpu_ticks >= hold_ticks {
                state.cpu_active = true;
                self.push_alert_event(
                    pid,
                    sample.process_name.clone(),
                    "cpu",
                    cpu,
                    self.alert_rules.cpu_pct,
                );
            }

            state.mem_ticks = if mem >= self.alert_rules.mem_pct {
                state.mem_ticks + 1
            } else {
                state.mem_active = false;
                0
            };
            if !state.mem_active && state.mem_ticks >= hold_ticks {
                state.mem_active = true;
                self.push_alert_event(
                    pid,
                    sample.process_name.clone(),
                    "mem",
                    mem,
                    self.alert_rules.mem_pct,
                );
            }

            if state.cpu_active || state.mem_active {
                active.insert(pid);
            }

            self.alert_state.insert(pid, state);
        }

        self.alert_state.retain(|pid, _| seen.contains(pid));
        self.active_alert_pids = active;
    }

    fn push_alert_event(
        &mut self,
        pid: u32,
        process_name: String,
        metric: &'static str,
        value: f32,
        threshold: f32,
    ) {
        let epoch_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if self.alert_log.len() >= 64 {
            let _ = self.alert_log.pop_front();
        }
        self.alert_log.push_back(AlertEvent {
            pid,
            process_name,
            metric,
            value,
            threshold,
            epoch_secs,
        });
    }

    pub fn is_pid_alerting(&self, pid: u32) -> bool {
        self.active_alert_pids.contains(&pid)
    }
}

pub async fn run<B: Backend + std::io::Write>(terminal: &mut Terminal<B>) -> Result<()> {
    let mut app = App::new()?;
    app.update_data();

    let ui_tick_ms = app.config.refresh_rate.clamp(8, 1000);
    let events = EventHandler::new(ui_tick_ms, 1000);
    terminal.draw(|f| ui::draw(f, &mut app))?;

    loop {
        let event = match events.rx.recv() {
            Ok(event) => event,
            Err(_) => break,
        };

        match event {
            Event::DataTick => {
                app.update_data();
            }
            Event::UiTick => {}
            Event::Key(key_event) => {
                app.handle_key(key_event);
            }
            Event::Resize(_, _) => {
                // Terminal resized, redraw on next frame.
            }
        }

        if app.should_quit {
            break;
        }

        terminal.draw(|f| ui::draw(f, &mut app))?;
    }

    app.config.sort_by = app.sort_by.as_str().to_string();
    let mut watchlist: Vec<u32> = app.watchlist.iter().copied().collect();
    watchlist.sort_unstable();
    app.config.watchlist_pids = watchlist;
    app.config.alerts_enabled = app.alert_rules.enabled;
    app.config.alert_cpu_pct = app.alert_rules.cpu_pct;
    app.config.alert_mem_pct = app.alert_rules.mem_pct;
    app.config.alert_hold_secs = app.alert_rules.hold_secs;
    app.config.save()?;
    Ok(())
}

fn push_capped<T>(history: &mut VecDeque<T>, value: T, cap: usize) {
    if history.len() >= cap {
        let _ = history.pop_front();
    }
    history.push_back(value);
}

fn compute_tree_visible_pids(
    processes: &[&sysinfo::Process],
    collapsed: &HashSet<u32>,
) -> Vec<u32> {
    let pid_set: HashSet<u32> = processes.iter().map(|p| p.pid().as_u32()).collect();
    let children = build_children_map(processes);
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

    let mut out = Vec::with_capacity(processes.len());

    fn walk(
        pid: u32,
        children: &HashMap<u32, Vec<u32>>,
        collapsed: &HashSet<u32>,
        out: &mut Vec<u32>,
    ) {
        out.push(pid);
        if collapsed.contains(&pid) {
            return;
        }
        if let Some(child_pids) = children.get(&pid) {
            for child in child_pids {
                walk(*child, children, collapsed, out);
            }
        }
    }

    for root in roots {
        walk(root, &children, collapsed, &mut out);
    }

    out
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
    for child_pids in children.values_mut() {
        child_pids.sort_unstable();
    }
    children
}
