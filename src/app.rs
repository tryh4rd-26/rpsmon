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
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppMode {
    Normal,
    Filter,
    Detail,
    SignalMenu,
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
    pub cpu_history: Vec<f32>,         // CPU usage history for graph (60 pts)
    pub mem_history: Vec<f32>,         // Memory usage history for graph (60 pts)
    pub net_history: Vec<(f64, f64)>,  // (rx_rate, tx_rate) in bytes/sec (60 pts)
    pub disk_io_history: Vec<(u64, u64)>,
    pub per_disk_history: HashMap<String, Vec<u64>>, 
    pub signal_input: String,
    pub signal_index: usize,             // Selection in the 1-31 signal grid
    pub selected_pid: Option<u32>,
    pub is_locked: bool,
    pub show_details: bool,
    pub tree_mode: bool,
    pub detail_tab: usize,              // 0=Identity, 1=Resources, 2=IO, 3=Relations
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
            sort_by: SortBy::Cpu,
            cpu_history: Vec::new(),
            mem_history: Vec::new(),
            net_history: Vec::with_capacity(60),
            disk_io_history: Vec::with_capacity(60),
            per_disk_history: HashMap::new(),
            selected_pid: None,
            is_locked: false,
            show_details: false,
            tree_mode: false,
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
        if !processes.is_empty() {
            if let Some(target_pid) = self.selected_pid {
                if self.is_locked {
                    if let Some(new_idx) =
                        processes.iter().position(|p| p.pid().as_u32() == target_pid)
                    {
                        self.selected_index = new_idx;
                    } else if self.selected_index >= processes.len() {
                        self.selected_index = processes.len() - 1;
                        self.selected_pid =
                            Some(processes[self.selected_index].pid().as_u32());
                    }
                } else {
                    if self.selected_index >= processes.len() {
                        self.selected_index = processes.len() - 1;
                    }
                    self.selected_pid =
                        Some(processes[self.selected_index].pid().as_u32());
                }
            } else {
                if self.selected_index >= processes.len() {
                    self.selected_index = processes.len() - 1;
                }
                self.selected_pid = Some(processes[self.selected_index].pid().as_u32());
            }
            self.table_state.select(Some(self.selected_index));
        } else {
            self.selected_index = 0;
            self.selected_pid = None;
            self.table_state.select(Some(0));
        }

        // ── CPU + Memory history ──
        if self.cpu_history.len() >= 60 {
            self.cpu_history.remove(0);
        }
        if self.mem_history.len() >= 60 {
            self.mem_history.remove(0);
        }
        let total = self.process_manager.total_memory();
        let used = self.process_manager.used_memory();
        let mem_ratio = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        self.cpu_history.push(self.process_manager.global_cpu_usage());
        self.mem_history.push(mem_ratio);

        // ── Network rate history ──
        let (rx_now, tx_now) = self.process_manager.get_network_stats();
        let rx_rate = rx_now.saturating_sub(self.prev_net.0) as f64;
        let tx_rate = tx_now.saturating_sub(self.prev_net.1) as f64;
        self.prev_net = (rx_now, tx_now);
        self.current_net_rate = (rx_rate, tx_rate);

        if self.net_history.len() >= 60 {
            self.net_history.remove(0);
        }
        self.net_history.push((rx_rate, tx_rate));

        // ── Disk I/O rate history ──
        let (disk_read, disk_write) = self.compute_total_disk_io();
        let read_rate = disk_read.saturating_sub(self.prev_disk_read);
        let write_rate = disk_write.saturating_sub(self.prev_disk_write);
        self.prev_disk_read = disk_read;
        self.prev_disk_write = disk_write;
        self.current_disk_rate = (read_rate as f64, write_rate as f64);

        if self.disk_io_history.len() >= 60 {
            self.disk_io_history.remove(0);
        }
        self.disk_io_history.push((read_rate, write_rate));
        
        // Track per-disk sparklines
        for disk in self.process_manager.get_disks() {
            let name = disk.filesystem.clone();
            let history = self.per_disk_history.entry(name).or_insert_with(|| Vec::with_capacity(60));
            if history.len() >= 60 {
                history.remove(0);
            }
            history.push(read_rate + write_rate);
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
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if !processes.is_empty() && self.selected_index < processes.len() - 1 {
            self.selected_index += 1;
            self.selected_pid = Some(processes[self.selected_index].pid().as_u32());
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn select_previous(&mut self) {
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if self.selected_index > 0 && !processes.is_empty() {
            self.selected_index -= 1;
            self.selected_pid = Some(processes[self.selected_index].pid().as_u32());
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn select_top(&mut self) {
        self.selected_index = 0;
        self.table_state.select(Some(0));
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if !processes.is_empty() {
            self.selected_pid = Some(processes[0].pid().as_u32());
        }
    }

    pub fn select_bottom(&mut self) {
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if !processes.is_empty() {
            self.selected_index = processes.len() - 1;
            self.selected_pid = Some(processes[self.selected_index].pid().as_u32());
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn send_custom_signal(&mut self, signal: i32) -> Result<()> {
        if let Some(pid) = self.selected_pid {
            self.process_manager.send_signal(sysinfo::Pid::from_u32(pid), signal)?;
            Ok(())
        } else {
            anyhow::bail!("No process selected")
        }
    }

    pub fn page_down(&mut self) {
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if !processes.is_empty() {
            self.selected_index = (self.selected_index + 20).min(processes.len() - 1);
            self.selected_pid = Some(processes[self.selected_index].pid().as_u32());
            self.table_state.select(Some(self.selected_index));
        }
    }

    pub fn page_up(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(20);
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if !processes.is_empty() && self.selected_index < processes.len() {
            self.selected_pid = Some(processes[self.selected_index].pid().as_u32());
        }
        self.table_state.select(Some(self.selected_index));
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
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if let Some(process) = processes.get(self.selected_index) {
            let _ = self.process_manager.kill_process(process.pid());
        }
    }

    pub fn sigkill_selected(&mut self) {
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if let Some(process) = processes.get(self.selected_index) {
            let _ = self.process_manager.send_signal(process.pid(), libc::SIGKILL);
        }
    }

    pub fn toggle_pause_selected(&mut self) {
        let processes = self
            .process_manager
            .get_filtered_and_sorted_processes(&self.search_query, self.sort_by);
        if let Some(process) = processes.get(self.selected_index) {
            let status = process.status();
            let sig = match status {
                sysinfo::ProcessStatus::Stop => libc::SIGCONT,
                _ => libc::SIGSTOP,
            };
            let _ = self.process_manager.send_signal(process.pid(), sig);
        }
    }

    pub fn cycle_sort(&mut self) {
        self.sort_by = self.sort_by.cycle();
        self.selected_index = 0;
        self.table_state.select(Some(0));
    }

    pub fn cycle_detail_tab(&mut self) {
        self.detail_tab = (self.detail_tab + 1) % 5; 
    }

    pub fn toggle_tree_mode(&mut self) {
        self.tree_mode = !self.tree_mode;
    }

    pub fn toggle_lock(&mut self) {
        self.is_locked = !self.is_locked;
    }
}

pub async fn run<B: Backend + std::io::Write>(terminal: &mut Terminal<B>) -> Result<()> {
    let mut app = App::new()?;
    let events = EventHandler::new();

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if let Ok(event) = events.rx.try_recv() {
            match event {
                Event::DataTick => {
                    app.update_data();
                }
                Event::UiTick => {
                    // Just rerender
                }
                Event::Key(key_event) => {
                    app.handle_key(key_event);
                }
                Event::Resize(_, _) => {
                    // Terminal resized, will redraw automatically
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    app.config.save()?;
    Ok(())
}
