use tokio::sync::broadcast;

use common::{message::ServerPayload, sprites::Sprite};

#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// 发送消息给指定玩家
    SendMessageToPlayer {
        player_id: u64,
        payload: ServerPayload,
    },
    PlayerReadyForMatchmaking {
        player_id: u64,
    },
    /// 匹配成功，通知系统创建对战房间
    MatchFound {
        players: [u64; 2],
    },
    /// 房间创建成功
    RoomCreated {
        room_id: u64,
        players: [u64; 2],
    },
    /// 关闭/销毁房间
    CloseRoom {
        room_id: u64,
    },
    /// 玩家准备好开始游戏
    RequestSpriteTeam {
        player_id: u64,
        room_id: u64,
    },
}

/// 事件总线
#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<ServerEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            sender: broadcast::channel(1024).0,
        }
    }

    /// 发布事件
    pub fn publish(&self, event: ServerEvent) {
        // 没有订阅者会发送失败，忽略错误
        let _ = self.sender.send(event);
    }

    /// 订阅事件
    pub fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.sender.subscribe()
    }
}
