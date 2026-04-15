use crate::app::{App, AppMode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct KeyHandler;

impl KeyHandler {
    pub fn handle(app: &mut App, key_event: KeyEvent) {
        // Global hotkeys that work in any mode
        match key_event.code {
            KeyCode::Char('t') | KeyCode::Char('T') => {
                app.toggle_tree_mode();
                return;
            }
            _ => {}
        }

        match app.mode {
            AppMode::Filter => {
                match key_event.code {
                    KeyCode::Esc => {
                        app.mode = AppMode::Normal;
                        app.search_query.clear();
                    }
                    KeyCode::Backspace => {
                        app.pop_search_char();
                    }
                    KeyCode::Char(c) => {
                        app.add_search_char(c);
                    }
                    KeyCode::Enter => {
                        app.mode = AppMode::Normal;
                    }
                    _ => {}
                }
            }
            AppMode::SignalMenu => {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.mode = AppMode::Detail;
                        app.signal_input.clear();
                    }
                    KeyCode::Up => {
                        if app.signal_index >= 4 { app.signal_index -= 4; }
                    }
                    KeyCode::Down => {
                        if app.signal_index + 4 < 31 { app.signal_index += 4; }
                    }
                    KeyCode::Left => {
                        if app.signal_index > 0 { app.signal_index -= 1; }
                    }
                    KeyCode::Right => {
                        if app.signal_index < 30 { app.signal_index += 1; }
                    }
                    KeyCode::Backspace => {
                        app.signal_input.pop();
                    }
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if app.signal_input.len() < 2 {
                            app.signal_input.push(c);
                            if let Ok(val) = app.signal_input.parse::<usize>() {
                                if val >= 1 && val <= 31 {
                                    app.signal_index = val - 1;
                                }
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let sig = (app.signal_index + 1) as i32;
                        let _ = app.send_custom_signal(sig);
                        app.mode = AppMode::Detail;
                        app.signal_input.clear();
                    }
                    _ => {}
                }
            }
            _ => {
                match key_event.code {
                    // ── Quit ──
                    KeyCode::Char('q') => {
                        if app.show_details {
                            app.show_details = false;
                            app.mode = AppMode::Normal;
                        } else {
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Esc => {
                        if app.show_details {
                            app.show_details = false;
                            app.mode = AppMode::Normal;
                        } else {
                            app.should_quit = true;
                        }
                    }
                    KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                        app.should_quit = true;
                    }

                    // ── Navigation ──
                    KeyCode::Char('j') | KeyCode::Down => {
                        app.select_next();
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        app.select_previous();
                    }
                    KeyCode::Char('g') => {
                        app.select_top();
                    }
                    KeyCode::Char('G') => {
                        app.select_bottom();
                    }
                    KeyCode::PageDown => {
                        app.page_down();
                    }
                    KeyCode::PageUp => {
                        app.page_up();
                    }

                    // ── Details ──
                    KeyCode::Enter => {
                        app.show_details = !app.show_details;
                        if app.show_details {
                            app.mode = AppMode::Detail;
                        } else {
                            app.mode = AppMode::Normal;
                        }
                    }
                    KeyCode::Tab => {
                        if app.show_details {
                            app.cycle_detail_tab();
                        }
                    }
                    KeyCode::Char('s') if app.show_details => {
                        app.mode = AppMode::SignalMenu;
                        app.signal_input.clear();
                        app.signal_index = 14; // Default to 15 (SIGTERM)
                    }
                    KeyCode::Char('s') if !app.show_details => {
                        app.cycle_sort();
                    }

                    // ── Filter/Search ──
                    KeyCode::Char('f') | KeyCode::Char('/') => {
                        app.mode = AppMode::Filter;
                    }

                    // ── Kill / Term / Pause ──
                    KeyCode::Char('v') => {
                        app.kill_selected();
                    }
                    KeyCode::Char('9') => {
                        app.sigkill_selected();
                    }
                    KeyCode::Char('p') => {
                        app.toggle_pause_selected();
                    }

                    _ => {}
                }
            }
        }
    }
}
