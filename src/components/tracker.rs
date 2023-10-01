#![allow(unused)]
use leptos::logging::log;

pub struct Tracker {
    id: usize,
}

impl Tracker {
    pub fn new(id: usize) -> Self {
        log!("Tracker {} created", id);
        Self { id }
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        log!("Tracker {} dropped", self.id);
    }
}
