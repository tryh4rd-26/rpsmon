use crate::app::App;
use crate::widgets;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    // ┌────────────── SYSTEM INFO BAR ──────────────────────┐
    // ├──────────┬─────────┬───────────────────────────────── ┤
    // │  CPU     │ Memory  │  Hardware + Network             │
    // ├──────────┴────┬────┴────────────────────────────────── ┤
    // │   Disk        │  Process List  │  System Info Panel  │
    // │               ├────────────────┤                     │
    // │               │  (Details)     │                     │
    // └───────────────┴────────────────┴──────────────────── ┘

    let main_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),     // System info bar
            Constraint::Percentage(45), // Top row: CPU + Mem + HW/Net
            Constraint::Percentage(55), // Bottom row: Disk + Processes + SysInfo
        ])
        .split(size);

    // ── System Info Bar ──
    widgets::sysinfo_bar::draw(f, app, main_vertical[0]);

    // ── Top Row: CPU | Memory | Hardware + Network ──
    let top_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // CPU
            Constraint::Percentage(30), // Memory
            Constraint::Percentage(35), // Hardware + Network stacked
        ])
        .split(main_vertical[1]);

    widgets::cpu::draw(f, app, top_row[0]);
    widgets::memory::draw(f, app, top_row[1]);

    // Hardware + Network stacked vertically in the right column
    let right_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Hardware
            Constraint::Percentage(50), // Network
        ])
        .split(top_row[2]);

    widgets::hardware::draw(f, app, right_col[0]);
    widgets::network::draw(f, app, right_col[1]);

    // ── Bottom Row: Disk | Processes | System Info ──
    let bottom_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Disk (reduced from 25%)
            Constraint::Percentage(50), // Processes (reduced from 75%)
            Constraint::Percentage(30), // New: System Info Panel
        ])
        .split(main_vertical[2]);

    widgets::disk::draw(f, app, bottom_row[0]);

    if app.show_details {
        // Split process area: list on top, details on bottom
        let proc_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(55), // Process list
                Constraint::Percentage(45), // Process details
            ])
            .split(bottom_row[1]);

        widgets::proc_list::draw(f, app, proc_split[0]);
        widgets::proc_detail::draw(f, app, proc_split[1]);
    } else {
        widgets::proc_list::draw(f, app, bottom_row[1]);
    }

    // ── System Info Panel (Right side) ──
    widgets::system_info::draw(f, app, bottom_row[2]);
}
