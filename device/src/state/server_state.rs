use crate::state::{
    query, Client, TickHistory, MESSAGE_QUERY, MESSAGE_SIZE, TICK_ALLOC, TICK_HISTORY_QUERY,
    TICK_HISTORY_RX_ALLOC, TICK_HISTORY_SIZE, TICK_QUERY, TICK_RX_ALLOC, TICK_SIZE,
};
use heapless::{String, Vec};
use log::debug;

type Tick = String<TICK_SIZE>;

pub struct ServerState {
    pub message: String<MESSAGE_SIZE>,
    pub ticks: Vec<Tick, TICK_ALLOC>,
    pub tick_history: Vec<TickHistory, TICK_HISTORY_SIZE>,
}

impl ServerState {
    pub async fn new<const WIFIRX: usize>(
        client: &mut Client<'_, '_, '_, '_, '_, WIFIRX>,
        response_buffer: &mut [u8],
    ) -> Self {
        let _raw_ticks: [u8; TICK_RX_ALLOC] = query(client, response_buffer, TICK_QUERY).await;

        Self {
            message: String::new(),
            ticks: Vec::new(),
            tick_history: Vec::new(),
        }
    }

    pub async fn update<const WIFIRX: usize>(
        &mut self,
        client: &mut Client<'_, '_, '_, '_, '_, WIFIRX>,
        response_buffer: &mut [u8],
    ) {
        let raw_message: [u8; MESSAGE_SIZE] = query(client, response_buffer, MESSAGE_QUERY).await;
        let message = core::str::from_utf8(&raw_message).unwrap();
        self.message = message.parse().unwrap();
        debug!("Message: {}", self.message);

        let raw_ticks: [u8; TICK_HISTORY_RX_ALLOC] =
            query(client, response_buffer, TICK_HISTORY_QUERY).await;
        let mut iterator = raw_ticks.iter();
        let size_bytes: [u8; 2] = [*iterator.next().unwrap(), *iterator.next().unwrap()];
        let size = u16::from_be_bytes(size_bytes);
        debug!("Tick History Size: {size}");
        self.tick_history.clear();
        for _ in 0..size {
            let tick = TickHistory::read(&mut iterator);
            debug!(
                "\n\tId: {}\n\tTime: {}:{}\n",
                tick.type_id, tick.time.hour, tick.time.minute
            );
            self.tick_history
                .push(tick)
                .expect("Too many ticks returned");
        }
    }
}
