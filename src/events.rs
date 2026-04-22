use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum Event {
    UiTick,   // High frequency for UI responsiveness (~60 FPS)
    DataTick, // Low frequency for system data collection (~1 FPS)
    Key(KeyEvent),
    Resize((), ()),
}

pub struct EventHandler {
    pub rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new(ui_tick_ms: u64, data_tick_ms: u64) -> Self {
        let (tx, rx) = mpsc::channel();

        // Key event thread
        let tx_key = tx.clone();
        thread::spawn(move || loop {
            if let Ok(true) = event::poll(Duration::from_millis(10)) {
                match event::read() {
                    Ok(CrosstermEvent::Key(key)) => {
                        if tx_key.send(Event::Key(key)).is_err() {
                            break;
                        }
                    }
                    Ok(CrosstermEvent::Resize(_, _)) => {
                        if tx_key.send(Event::Resize((), ())).is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        });

        // UI tick thread
        let tx_ui = tx.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(ui_tick_ms.max(8)));
            if tx_ui.send(Event::UiTick).is_err() {
                break;
            }
        });

        // Data tick thread
        let tx_data = tx;
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(data_tick_ms.max(250)));
            if tx_data.send(Event::DataTick).is_err() {
                break;
            }
        });

        Self { rx }
    }
}
