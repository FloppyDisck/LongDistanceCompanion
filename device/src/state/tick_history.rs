use crate::state::Time;
use core::slice::Iter;

#[derive(Debug)]
pub struct TickHistory {
    pub type_id: u8,
    pub time: Time,
}

impl TickHistory {
    pub fn read(reader: &mut Iter<u8>) -> Self {
        let type_id = *reader.next().unwrap();
        let time = Time::read(reader);
        Self { type_id, time }
    }
}
