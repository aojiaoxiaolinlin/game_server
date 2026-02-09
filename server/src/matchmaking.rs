use crate::events::{EventBus, ServerEvent};

pub struct MatchmakingService {
    event_bus: EventBus,
    /// 匹配队列
    queue: Vec<u64>,
}

impl MatchmakingService {
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            event_bus,
            queue: Vec::new(),
        }
    }

    pub async fn run(&mut self) {
        let mut event_receiver = self.event_bus.subscribe();
        loop {
            if let Ok(ServerEvent::PlayerReadyForMatchmaking { player_id }) =
                event_receiver.recv().await
            {
                self.queue.push(player_id);
                self.try_create_match();
            }
        }
    }

    fn try_create_match(&mut self) {
        if self.queue.len() >= 2 {
            let player_a = self.queue.remove(0);
            let player_b = self.queue.remove(0);

            println!("[Matchmaking] Match found: {} vs {}", player_a, player_b);
            // 匹配成功，通知系统创建对战房间
            self.event_bus.publish(ServerEvent::MatchFound {
                players: [player_a, player_b],
            });
        }
    }
}
