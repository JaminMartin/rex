use color_eyre::eyre::Result;
use crossterm::event::KeyEvent;
use futures::{FutureExt, StreamExt};
use tokio::{sync::mpsc, task::JoinHandle};

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Error,
    Tick,
    Key(KeyEvent),
}

pub struct EventHandler {
    _tx: mpsc::UnboundedSender<Event>,
    rx: mpsc::UnboundedReceiver<Event>,
    _task: JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate_ms: u64) -> Self {
        let tick_rate = std::time::Duration::from_millis(tick_rate_ms);
        let (tx, rx) = mpsc::unbounded_channel();
        let _tx = tx.clone();

        let task = tokio::spawn(async move {
            let mut reader = crossterm::event::EventStream::new();
            let mut interval = tokio::time::interval(tick_rate);

            loop {
                let delay = interval.tick();
                let crossterm_event = reader.next().fuse();

                tokio::select! {
                    maybe_event = crossterm_event => {
                        match maybe_event {
                            Some(Ok(evt)) => {
                                match evt {
                                    crossterm::event::Event::Key(key) => {
                                        if key.kind == crossterm::event::KeyEventKind::Press {
                                            let _ = tx.send(Event::Key(key));
                                        }
                                    },
                                    _ => {},
                                }
                            }
                            Some(Err(_)) => {
                                let _ = tx.send(Event::Error);
                            }
                            None => {},
                        }
                    },
                    _ = delay => {
                        let _ = tx.send(Event::Tick);
                    },
                }
            }
        });

        Self {
            _tx,
            rx,
            _task: task,
        }
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or(color_eyre::eyre::eyre!("Unable to get event"))
    }
}
