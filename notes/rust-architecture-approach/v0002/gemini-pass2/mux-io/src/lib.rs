use mio::{Poll, Events, Token, Interest};
use std::time::Duration;

pub struct EventLoop {
    poll: Poll,
    events: Events,
}

impl EventLoop {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            poll: Poll::new()?,
            events: Events::with_capacity(1024),
        })
    }

    pub fn run<F>(&mut self, mut handler: F) -> std::io::Result<()>
    where
        F: FnMut(&mio::event::Event),
    {
        loop {
            self.poll.poll(&mut self.events, Some(Duration::from_millis(100)))?;
            for event in &self.events {
                handler(event);
            }
        }
    }
}
