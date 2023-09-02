#![allow(unused)]
pub struct Tracker {
    id: usize,
}

impl Tracker {
    pub fn new(id: usize) -> Self {
        leptos::log!("Tracker {} created", id);
        Self { id }
    }
}

impl Drop for Tracker {
    fn drop(&mut self) {
        leptos::log!("Tracker {} dropped", self.id);
    }
}
