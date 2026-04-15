use sysinfo::{System, Pid, Process, Networks, Components, Users};
use crate::sort::SortBy;
use std::collections::HashMap;
use std::process::Command;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct DiskInfo {
    pub filesystem: String,
    pub total_space: u64,
    pub used_space: u64,
    pub available_space: u64,
    pub mount_point: PathBuf,
}

#[allow(dead_code)]
pub struct SystemMetrics {
    pub cpu_usage_per_core: Vec<f32>,
    pub temp: f32,
    pub fan_speed: u32,
}

pub struct ProcessManager {
    sys: System,
    pub networks: Networks,
    pub components: Components,
    pub users: Users,
    pub iface_ips: HashMap<String, Vec<String>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let users = Users::new_with_refreshed_list();
        let mut pm = Self {
            sys,
            networks,
            components,
            users,
            iface_ips: HashMap::new(),
        };
        pm.update_ips();
        pm
    }

    pub fn sys(&self) -> &System {
        &self.sys
    }

    pub fn refresh(&mut self) {
        self.sys.refresh_all();
        self.users.refresh_list();
        self.networks.refresh();
        self.components.refresh_list();
        self.update_ips();
    }

    fn update_ips(&mut self) {
        self.iface_ips.clear();
        
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("ifconfig").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut current_iface = String::new();
                for line in stdout.lines() {
                    if !line.starts_with('\t') && line.contains(':') {
                        current_iface = line.split(':').next().unwrap_or("").to_string();
                    } else if !current_iface.is_empty() && line.contains("inet ") {
                        let ip = line.split_whitespace().nth(1).unwrap_or("").to_string();
                        self.iface_ips.entry(current_iface.clone()).or_default().push(ip);
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("ip").arg("addr").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut current_iface = String::new();
                for line in stdout.lines() {
                    if !line.starts_with(' ') && line.contains(": ") {
                        current_iface = line.split(": ").nth(1).unwrap_or("").to_string();
                    } else if !current_iface.is_empty() && line.contains("inet ") {
                        let ip = line.split_whitespace().nth(1).split('/').next().unwrap_or("").to_string();
                        self.iface_ips.entry(current_iface.clone()).or_default().push(ip);
                    }
                }
            }
        }
    }

    pub fn get_network_stats(&self) -> (u64, u64) {
        let mut rx = 0;
        let mut tx = 0;
        for (_, data) in &self.networks {
            rx += data.received();
            tx += data.transmitted();
        }
        (rx, tx)
    }

    pub fn uptime(&self) -> u64 {
        System::uptime()
    }

    pub fn hostname(&self) -> String {
        System::host_name().unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn os_version(&self) -> String {
        System::os_version().unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn kernel_version(&self) -> String {
        System::kernel_version().unwrap_or_else(|| "Unknown".to_string())
    }

    pub fn load_average(&self) -> String {
        let load = System::load_average();
        format!("{:.2} {:.2} {:.2}", load.one, load.five, load.fifteen)
    }

    pub fn get_thermal(&self) -> f32 {
        let mut max_temp: f32 = 0.0;
        for component in &self.components {
            if component.temperature() > max_temp {
                max_temp = component.temperature();
            }
        }
        max_temp
    }

    pub fn get_battery(&self) -> String {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("pmset").arg("-g").arg("batt").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(pct_str) = stdout.split('%').next() {
                    if let Some(pos) = pct_str.rfind(|c: char| c.is_ascii_digit()) {
                        let mut start = pos;
                        while start > 0 && pct_str.chars().nth(start - 1).unwrap_or(' ').is_ascii_digit() {
                            start -= 1;
                        }
                        return format!("{}%", &pct_str[start..=pos]);
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Try common battery paths on Linux
            for bat in &["BAT0", "BAT1", "battery"] {
                let path = format!("/sys/class/power_supply/{}/capacity", bat);
                if let Ok(cap) = std::fs::read_to_string(path) {
                    return format!("{}%", cap.trim());
                }
            }
        }

        "100%".to_string()
    }

    pub fn get_gpu(&self) -> String {
        // Try nvidia-smi first
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .arg("--query-gpu=utilization.gpu")
            .arg("--format=csv,noheader,nounits")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !stdout.is_empty() {
                return format!("{}%", stdout);
            }
        }
        "No NVIDIA GPU detected".to_string()
    }

    pub fn get_all_processes(&self) -> Vec<&Process> {
        self.sys.processes().values().collect()
    }

    pub fn get_filtered_processes(&self, query: &str) -> Vec<&Process> {
        let query_lower = query.to_lowercase();

        self.get_all_processes()
            .into_iter()
            .filter(|p| {
                if query.is_empty() {
                    return true;
                }

                let pid_str = p.pid().as_u32().to_string();
                let name = p.name().to_lowercase();
                let user = p.user_id()
                    .map(|uid| uid.to_string())
                    .unwrap_or_default()
                    .to_lowercase();

                pid_str.contains(&query_lower)
                    || name.contains(&query_lower)
                    || user.contains(&query_lower)
            })
            .collect()
    }

    pub fn get_filtered_and_sorted_processes(&self, query: &str, sort_by: SortBy) -> Vec<&Process> {
        let mut processes = self.get_filtered_processes(query);
        let cpu_count = self.cpu_count() as f32;

        match sort_by {
            SortBy::Pid => processes.sort_by_key(|p| std::cmp::Reverse(p.pid().as_u32())),
            SortBy::Name => processes.sort_by(|a, b| a.name().cmp(b.name())),
            SortBy::Cpu => processes.sort_by(|a, b| {
                let a_cpu = a.cpu_usage() / cpu_count;
                let b_cpu = b.cpu_usage() / cpu_count;
                b_cpu.partial_cmp(&a_cpu).unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortBy::Memory => processes.sort_by(|a, b| {
                b.memory().cmp(&a.memory())
            }),
            SortBy::Threads => {
                processes.sort_by(|a, b| {
                    let a_t = a.tasks().map(|t| t.len()).unwrap_or(1);
                    let b_t = b.tasks().map(|t| t.len()).unwrap_or(1);
                    b_t.cmp(&a_t)
                })
            }
        }

        processes
    }

    pub fn kill_process(&self, pid: Pid) -> std::io::Result<()> {
        self.send_signal(pid, libc::SIGTERM)
    }

    pub fn send_signal(&self, pid: Pid, signal: i32) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            unsafe {
                if libc::kill(pid.as_u32() as i32, signal) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            }
        }

        #[cfg(not(unix))]
        {
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Signals not supported on this platform",
            ))
        }
    }

    pub fn total_memory(&self) -> u64 {
        self.sys.total_memory()
    }

    pub fn used_memory(&self) -> u64 {
        self.sys.used_memory()
    }

    pub fn available_memory(&self) -> u64 {
        self.sys.available_memory()
    }

    pub fn free_memory(&self) -> u64 {
        self.sys.free_memory()
    }

    pub fn total_swap(&self) -> u64 {
        self.sys.total_swap()
    }

    pub fn used_swap(&self) -> u64 {
        self.sys.used_swap()
    }

    pub fn get_disks(&self) -> Vec<DiskInfo> {
        self.parse_df_output()
    }

    fn parse_df_output(&self) -> Vec<DiskInfo> {
        let mut disks = Vec::new();

        if let Ok(output) = Command::new("df").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut lines = stdout.lines();
            
            // Skip header line
            lines.next();
            
            for line in lines {
                let parts: Vec<&str> = line.split_whitespace().collect();
                // df format: Filesystem 512-blocks Used Available Capacity iused ifree %iused Mounted on
                // parts: [0]filesystem [1]blocks [2]used [3]available [4]capacity [5]iused [6]ifree [7]%iused [8+]mounted_on
                if parts.len() >= 9 {
                    if let (Ok(total), Ok(used), Ok(avail)) = (
                        parts[1].parse::<u64>(),
                        parts[2].parse::<u64>(),
                        parts[3].parse::<u64>(),
                    ) {
                        // Convert 512-byte blocks to bytes
                        let total_bytes = total * 512;
                        let used_bytes = used * 512;
                        let avail_bytes = avail * 512;
                        
                        let mount_point = parts[8..].join(" ");
                        
                        disks.push(DiskInfo {
                            filesystem: parts[0].to_string(),
                            total_space: total_bytes,
                            used_space: used_bytes,
                            available_space: avail_bytes,
                            mount_point: PathBuf::from(mount_point),
                        });
                    }
                }
            }
        }

        disks
    }

    pub fn global_cpu_usage(&self) -> f32 {
        self.sys.global_cpu_info().cpu_usage()
    }

    pub fn cpu_count(&self) -> usize {
        self.sys.cpus().len()
    }

    pub fn get_cpu_usages(&self) -> Vec<f32> {
        self.sys.cpus().iter().map(|cpu| cpu.cpu_usage()).collect()
    }

    pub fn get_cpu_name(&self) -> String {
        self.sys.global_cpu_info().brand().to_string()
    }

    pub fn get_cpu_brand(&self) -> String {
        self.sys.global_cpu_info().vendor_id().to_string()
    }

    /// Get memory cache (Linux-specific)
    pub fn get_mem_cache(&self) -> u64 {
        // Try to read from /proc/meminfo on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("Cached:") {
                        if let Some(val_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = val_str.parse::<u64>() {
                                return kb * 1024;
                            }
                        }
                    }
                }
            }
        }
        0
    }

    /// Get memory buffers (Linux-specific)
    pub fn get_mem_buffers(&self) -> u64 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("Buffers:") {
                        if let Some(val_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = val_str.parse::<u64>() {
                                return kb * 1024;
                            }
                        }
                    }
                }
            }
        }
        0
    }

    /// Get SReclaimable memory (Linux-specific - part of slab)
    pub fn get_mem_sreclaimable(&self) -> u64 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("SReclaimable:") {
                        if let Some(val_str) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = val_str.parse::<u64>() {
                                return kb * 1024;
                            }
                        }
                    }
                }
            }
        }
        0
    }

    // ── Advanced Memory Metrics (macOS via vm_stat) ──
    pub fn get_mem_wired(&self) -> u64 {
        self.parse_vm_stat("Pages wired down").unwrap_or(0)
    }

    pub fn get_mem_compressed(&self) -> u64 {
        self.parse_vm_stat("Pages occupied by compressor").unwrap_or(0)
    }

    pub fn get_mem_purgeable(&self) -> u64 {
        self.parse_vm_stat("Pages purgeable").unwrap_or(0)
    }

    pub fn get_mem_anonymous(&self) -> u64 {
        self.parse_vm_stat("Anonymous pages").unwrap_or(0)
    }

    pub fn get_mem_file_backed(&self) -> u64 {
        self.parse_vm_stat("File-backed pages").unwrap_or(0)
    }

    pub fn get_mem_app(&self) -> u64 {
        // App memory is roughly: total - wired - compressed
        let total = self.total_memory();
        let wired = self.get_mem_wired() * 4096; // Convert pages to bytes
        let compressed = self.get_mem_compressed() * 4096;
        total.saturating_sub(wired).saturating_sub(compressed)
    }

    fn parse_vm_stat(&self, key: &str) -> Option<u64> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("vm_stat").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains(key) {
                        if let Some(val_str) = line.split_whitespace().last() {
                            let val_str = val_str.trim_end_matches('.');
                            if let Ok(val) = val_str.parse::<u64>() {
                                return Some(val);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    // ── System Statistics ──
    pub fn get_total_threads(&self) -> usize {
        self.get_all_processes().iter().map(|p| p.tasks().map(|t| t.len()).unwrap_or(1)).sum()
    }

    pub fn get_open_fds(&self) -> u64 {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("lsof").arg("-p").arg(std::process::id().to_string()).output() {
                return output.stdout.len() as u64 / 100; // Rough estimate
            }
        }
        #[cfg(target_os = "linux")]
        {
            if let Ok(entries) = std::fs::read_dir("/proc/self/fd") {
                return entries.count() as u64;
            }
        }
        0
    }

    pub fn get_zombie_count(&self) -> usize {
        self.get_all_processes().iter().filter(|p| p.status().to_string().contains("Zombie")).count()
    }

    pub fn get_daemon_count(&self) -> usize {
        self.get_all_processes().iter().filter(|p| p.name().contains("d") && p.name().ends_with("d")).count()
    }

    pub fn get_context_switches(&self) -> u64 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/stat") {
                for line in content.lines() {
                    if line.starts_with("ctxt ") {
                        if let Some(val_str) = line.split_whitespace().nth(1) {
                            if let Ok(val) = val_str.parse::<u64>() {
                                return val;
                            }
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = Command::new("sysctl").arg("-n").arg("vm.vm_stat").output() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(line) = stdout.lines().find(|l| l.contains("context switches")) {
                    if let Some(val_str) = line.split_whitespace().last() {
                        if let Ok(val) = val_str.parse::<u64>() {
                            return val;
                        }
                    }
                }
            }
        }
        0
    }

    pub fn get_interrupts(&self) -> u64 {
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/stat") {
                if let Some(line) = content.lines().next() {
                    if line.starts_with("intr ") {
                        if let Some(val_str) = line.split_whitespace().nth(1) {
                            if let Ok(val) = val_str.parse::<u64>() {
                                return val;
                            }
                        }
                    }
                }
            }
        }
        0
    }

    pub fn get_avg_task_memory(&self) -> u64 {
        let procs = self.get_all_processes();
        if procs.is_empty() {
            return 0;
        }
        let total_mem: u64 = procs.iter().map(|p| p.memory()).sum();
        total_mem / procs.len() as u64
    }

    pub fn get_user_cpu_pct(&self) -> f32 {
        // Simplified: sum all process CPU usage that's user-space
        let procs = self.get_all_processes();
        if procs.is_empty() {
            return 0.0;
        }
        let total_cpu: f32 = procs.iter().map(|p| p.cpu_usage()).sum();
        (total_cpu / (self.cpu_count() as f32 * 100.0) * 100.0).min(100.0)
    }
}
