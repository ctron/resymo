use homeassistant_agent::model::{Availability, AvailabilityMode, Discovery};
use std::fmt::Display;

pub trait MixinAvailability: Sized {
    fn mixin_availability(self, base: impl Display, global: impl Into<String>) -> Self;
}

impl MixinAvailability for Discovery {
    fn mixin_availability(mut self, base: impl Display, global: impl Into<String>) -> Self {
        for entry in &mut self.availability {
            entry.topic = format!("{base}/{}", entry.topic);
        }
        self.availability.push(Availability::new(global));
        self.availability_mode = AvailabilityMode::All;

        self
    }
}
