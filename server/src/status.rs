#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerSession {
    // ... other fields like player_id, stream
    player_id: u64,
    last_client_sequence: u64,
    server_sequence: u64,
}

impl PlayerSession {
    pub fn new(player_id: u64, last_client_sequence: u64, server_sequence: u64) -> Self {
        Self {
            player_id,
            last_client_sequence,
            server_sequence,
        }
    }

    pub fn player_id(&self) -> u64 {
        self.player_id
    }

    pub fn last_client_sequence(&self) -> u64 {
        self.last_client_sequence
    }

    pub fn server_sequence(&self) -> u64 {
        self.server_sequence
    }

    pub fn last_client_sequence_increment(&mut self) {
        self.last_client_sequence += 1;
    }

    pub fn server_sequence_increment(&mut self) {
        self.server_sequence += 1;
    }
}
