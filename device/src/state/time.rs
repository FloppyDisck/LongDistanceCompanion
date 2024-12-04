use core::slice::Iter;

#[derive(Debug)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
}

impl Time {
    pub fn read(reader: &mut Iter<u8>) -> Self {
        let hour = *reader.next().unwrap();
        let minute = *reader.next().unwrap();
        Self { hour, minute }
    }
}
