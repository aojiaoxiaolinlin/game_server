use crate::{
    actor::{ActorMessage, SystemMessage},
    events::{EventBus, ServerEvent},
    room_manager::RoomManager,
    session::SessionManager,
};

/// 游戏协调器
/// 负责协调游戏中的各种操作，如创建房间、关闭房间
pub struct GameCoordinator {
    room_manager: RoomManager,
    session_manager: SessionManager,
    event_bus: EventBus,
}

impl GameCoordinator {
    pub fn new(
        room_manager: RoomManager,
        session_manager: SessionManager,
        event_bus: EventBus,
    ) -> Self {
        Self {
            room_manager,
            session_manager,
            event_bus,
        }
    }

    pub async fn run(&mut self) {
        println!("游戏协调器启动");
        let mut recv = self.event_bus.subscribe();
        while let Ok(event) = recv.recv().await {
            match event {
                ServerEvent::MatchFound { players } => {
                    println!("接收到 MatchFound 事件 player {:?}", players);
                    self.handle_match(players).await;
                }
                ServerEvent::CloseRoom { room_id } => {
                    self.handle_close_room(room_id).await;
                }

                _ => {}
            }
        }
    }

    async fn handle_match(&self, players: [u64; 2]) {
        let room_id = self
            .room_manager
            .create_room(
                players,
                self.event_bus.clone(),
                self.session_manager.clone(),
            )
            .await;
        println!("创建房间 {} 成功", room_id);

        for player_id in &players {
            if let Some(player_handle) = self.session_manager.get_session(*player_id).await {
                // TODO: 获取对手信息
                let opponent_name = "Opponent".to_string();

                let notification = ActorMessage::SystemNotification(SystemMessage::EnterRoom {
                    room_id,
                    opponent_name,
                });
                println!("通知玩家 {} 进入房间 {}", player_id, room_id);

                if player_handle.send(notification).await.is_err() {
                    // 玩家可能刚好掉线了, 需要处理这种失败情况
                    println!(
                        "[GameCoordinator] Failed to notify player {}, they might have disconnected.",
                        player_id
                    );
                    // TODO: 需要通知 RoomActor 有玩家连接失败
                }
            }
        }
        self.event_bus
            .publish(ServerEvent::RoomCreated { room_id, players });
    }

    async fn handle_close_room(&self, room_id: u64) {
        self.room_manager.remove_room(room_id).await;
        println!("关闭房间 {} 成功", room_id);
    }
}
