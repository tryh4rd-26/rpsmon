use crossterm::event::{self, KeyEvent, Event as CrosstermEvent};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[derive(Clone, Debug)]
pub enum Event {
    UiTick,          // High frequency for UI responsiveness (~60 FPS)
    DataTick,        // Low frequency for system data collection (~1 FPS)
    Key(KeyEvent),
    Resize((), ()),
}

pub struct EventHandler {
    pub rx: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        // Key event thread
        let tx_key = tx.clone();
        thread::spawn(move || {
            loop {
                if let Ok(true) = event::poll(Duration::from_millis(10)) {
                    if let Ok(CrosstermEvent::Key(key)) = event::read() {
                        let _ = tx_key.send(Event::Key(key));
                    } else if let Ok(CrosstermEvent::Resize(_, _)) = event::read() {
                        let _ = tx_key.send(Event::Resize((), ()));
                    }
                }
            }
        });

        // UI tick thread (20 FPS for efficiency)
        let tx_ui = tx.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(50));
                let _ = tx_ui.send(Event::UiTick);
            }
        });

        // Data tick thread (1 FPS for stable process displays)
        let tx_data = tx;
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(1000));
                let _ = tx_data.send(Event::DataTick);
            }
        });

        Self { rx }
    }
}
