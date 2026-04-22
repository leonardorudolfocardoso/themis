use crate::event::Event;

#[derive(Default)]
pub(crate) struct EventLog {
    events: Vec<Event>,
}

impl EventLog {
    pub(crate) fn append(&mut self, event: Event) {
        self.events.push(event);
    }

    pub(crate) fn events(&self) -> &[Event] {
        &self.events
    }
}
