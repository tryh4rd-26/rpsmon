use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Color palette and gradient system for the entire application.
/// Every panel gets its own distinct multi-colored gradient.

// ── Panel border colors ──────────────────────────────────────────
pub const BORDER_CPU: Color = Color::Rgb(0, 210, 140);     // Emerald green
pub const BORDER_MEM: Color = Color::Rgb(180, 80, 255);    // Vibrant purple
pub const BORDER_DISK: Color = Color::Rgb(255, 160, 40);   // Warm orange
pub const BORDER_NET: Color = Color::Rgb(0, 180, 220);     // Teal
pub const BORDER_HW: Color = Color::Rgb(255, 100, 100);    // Coral red
pub const BORDER_PROC: Color = Color::Rgb(80, 140, 255);   // Sky blue
pub const BORDER_DETAIL: Color = Color::Rgb(255, 200, 60); // Gold
pub const BORDER_INFO: Color = Color::Rgb(120, 200, 255);  // Light cyan

// ── Text colors ──────────────────────────────────────────────────
pub const TEXT_LABEL: Color = Color::Rgb(100, 110, 130);    // Muted label
pub const TEXT_VALUE: Color = Color::Rgb(200, 210, 225);    // Bright value
pub const TEXT_ACCENT: Color = Color::Rgb(130, 220, 255);   // Cyan accent
pub const TEXT_DIM: Color = Color::Rgb(70, 78, 94);         // Very dim
pub const TEXT_BRIGHT: Color = Color::Rgb(240, 245, 255);   // Near white
pub const TEXT_SECTION: Color = Color::Rgb(255, 200, 60);   // Gold section header

// ── Header bar colors ───────────────────────────────────────────
pub const HEADER_BG: Color = Color::Rgb(25, 28, 38);
pub const HEADER_TITLE: Color = Color::Rgb(130, 220, 255);
pub const HEADER_VALUE: Color = Color::Rgb(200, 210, 225);

// ── Process list row coloring ───────────────────────────────────
pub const PROC_LOW: Color = Color::Rgb(120, 180, 240);     // Light blue - low usage
pub const PROC_MED: Color = Color::Rgb(255, 220, 100);     // Amber - medium usage
pub const PROC_HIGH: Color = Color::Rgb(255, 140, 70);     // Orange - high usage
pub const PROC_CRIT: Color = Color::Rgb(255, 80, 80);      // Red - critical usage
pub const PROC_SELECTED_BG: Color = Color::Rgb(40, 60, 100); // Highlighted row bg
pub const PROC_SELECTED_FG: Color = Color::Rgb(255, 255, 255);

// ── Bar background ──────────────────────────────────────────────
pub const BAR_BG: Color = Color::Rgb(35, 40, 55);

// ── Memory breakdown colors ─────────────────────────────────────
pub const COLOR_ORANGE: Color = Color::Rgb(255, 160, 40);   // Orange - available
pub const COLOR_BLUE: Color = Color::Rgb(80, 140, 255);     // Blue - cached

/// CPU usage → color (green → yellow → orange → red)
pub fn cpu_color(pct: f32) -> Color {
    if pct > 90.0 {
        Color::Rgb(255, 50, 50)    // Bright red
    } else if pct > 75.0 {
        Color::Rgb(255, 100, 50)   // Orange-red
    } else if pct > 60.0 {
        Color::Rgb(255, 160, 40)   // Orange
    } else if pct > 40.0 {
        Color::Rgb(255, 220, 60)   // Yellow
    } else if pct > 20.0 {
        Color::Rgb(140, 230, 80)   // Yellow-green
    } else {
        Color::Rgb(60, 210, 120)   // Green
    }
}

/// Memory usage → color (cyan → blue → magenta → red)
pub fn mem_color(pct: f32) -> Color {
    if pct > 90.0 {
        Color::Rgb(255, 60, 80)    // Red
    } else if pct > 75.0 {
        Color::Rgb(220, 80, 200)   // Magenta
    } else if pct > 50.0 {
        Color::Rgb(160, 100, 255)  // Purple
    } else if pct > 25.0 {
        Color::Rgb(80, 140, 255)   // Blue
    } else {
        Color::Rgb(80, 210, 230)   // Cyan
    }
}

/// Temperature → color (blue cool → green → yellow → red hot)
pub fn temp_color(temp: f32) -> Color {
    if temp > 90.0 {
        Color::Rgb(255, 40, 40)
    } else if temp > 75.0 {
        Color::Rgb(255, 140, 40)
    } else if temp > 60.0 {
        Color::Rgb(255, 220, 60)
    } else if temp > 40.0 {
        Color::Rgb(100, 220, 100)
    } else {
        Color::Rgb(80, 180, 255)
    }
}

/// Disk usage → color (blue → yellow → red)
pub fn disk_color(pct: f32) -> Color {
    if pct > 90.0 {
        Color::Rgb(255, 50, 50)
    } else if pct > 75.0 {
        Color::Rgb(255, 160, 40)
    } else if pct > 50.0 {
        Color::Rgb(255, 220, 80)
    } else {
        Color::Rgb(80, 160, 255)
    }
}

/// Network Rx color
pub const NET_RX: Color = Color::Rgb(80, 220, 200);  // Teal
/// Network Tx color
pub const NET_TX: Color = Color::Rgb(255, 140, 80);  // Orange

/// Battery level → color
pub fn battery_color(pct: f32) -> Color {
    if pct > 60.0 {
        Color::Rgb(80, 220, 120)
    } else if pct > 30.0 {
        Color::Rgb(255, 220, 60)
    } else if pct > 15.0 {
        Color::Rgb(255, 140, 40)
    } else {
        Color::Rgb(255, 50, 50)
    }
}

/// Process row color based on combined CPU + MEM intensity
pub fn process_row_color(cpu_pct: f32, mem_mb: f32) -> Color {
    if cpu_pct > 20.0 || mem_mb > 2000.0 {
        PROC_CRIT
    } else if cpu_pct > 10.0 || mem_mb > 1000.0 {
        PROC_HIGH
    } else if cpu_pct > 5.0 || mem_mb > 500.0 {
        PROC_MED
    } else {
        PROC_LOW
    }
}

/// Heatmap color for per-core mini display (8-step ramp)
pub fn heatmap_color(pct: f32) -> Color {
    match pct as u32 {
        0..=12  => Color::Rgb(30, 60, 90),
        13..=25 => Color::Rgb(40, 120, 140),
        26..=37 => Color::Rgb(60, 180, 120),
        38..=50 => Color::Rgb(140, 220, 60),
        51..=62 => Color::Rgb(220, 220, 40),
        63..=75 => Color::Rgb(255, 180, 30),
        76..=87 => Color::Rgb(255, 120, 40),
        _       => Color::Rgb(255, 50, 50),
    }
}

/// Gauge bar colors for different categories
pub const GAUGE_RAM: Color = Color::Rgb(180, 80, 255);
pub const GAUGE_SWAP: Color = Color::Rgb(255, 100, 100);
pub const GAUGE_CACHE: Color = Color::Rgb(80, 200, 200);
pub const GAUGE_BUFFER: Color = Color::Rgb(120, 180, 60);

/// Detail tab highlight
pub const TAB_ACTIVE: Color = Color::Rgb(255, 200, 60);
pub const TAB_INACTIVE: Color = Color::Rgb(80, 90, 110);

/// Status indicator colors
pub const STATUS_RUN: Color = Color::Rgb(80, 220, 120);
pub const STATUS_SLEEP: Color = Color::Rgb(80, 140, 255);
pub const STATUS_ZOMBIE: Color = Color::Rgb(255, 60, 60);
pub const STATUS_STOP: Color = Color::Rgb(255, 200, 60);

/// Create a monochromatic gradient progress bar that fades from bright to dark
pub fn gradient_bar(pct: f32, width: u16, base_color: Color) -> Line<'static> {
    let mut spans = Vec::new();
    let filled = (pct / 100.0 * width as f32).min(width as f32) as u16;

    for i in 0..width {
        let symbol = if i < filled { "█" } else { "░" };
        let color = if i < filled {
            // Fade the color across the width (brightest at start or end? Btop usually fades left to right)
            let ratio = 1.0 - (i as f32 / width as f32) * 0.7; // Fade to 30% intensity
            fade_color(base_color, ratio)
        } else {
            BAR_BG
        };
        spans.push(Span::styled(symbol, Style::default().fg(color)));
    }

    Line::from(spans)
}

pub fn fade_color(color: Color, factor: f32) -> Color {
    if let Color::Rgb(r, g, b) = color {
        // Ensure even the most faded color has a floor of ~20% of original but at least some brightness
        let floor = 0.2;
        let final_factor = factor.max(floor);
        Color::Rgb(
            (r as f32 * final_factor).min(255.0) as u8,
            (g as f32 * final_factor).min(255.0) as u8,
            (b as f32 * final_factor).min(255.0) as u8,
        )
    } else {
        color
    }
}

/// Process status → color
pub fn status_color(status: sysinfo::ProcessStatus) -> Color {
    match status {
        sysinfo::ProcessStatus::Run => STATUS_RUN,
        sysinfo::ProcessStatus::Sleep => STATUS_SLEEP,
        sysinfo::ProcessStatus::Zombie => STATUS_ZOMBIE,
        sysinfo::ProcessStatus::Stop => STATUS_STOP,
        sysinfo::ProcessStatus::Tracing => STATUS_STOP,
        sysinfo::ProcessStatus::Dead => STATUS_ZOMBIE,
        sysinfo::ProcessStatus::Wakekill => STATUS_STOP,
        sysinfo::ProcessStatus::Waking => STATUS_RUN,
        sysinfo::ProcessStatus::Parked => STATUS_SLEEP,
        _ => STATUS_SLEEP,
    }
}

fn interpolate_color(c1: Color, c2: Color, ratio: f32) -> Color {
    if let (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) = (c1, c2) {
        let r = (r1 as f32 + (r2 as f32 - r1 as f32) * ratio) as u8;
        let g = (g1 as f32 + (g2 as f32 - g1 as f32) * ratio) as u8;
        let b = (b1 as f32 + (b2 as f32 - b1 as f32) * ratio) as u8;
        Color::Rgb(r, g, b)
    } else {
        c1 // Fallback
    }
}
