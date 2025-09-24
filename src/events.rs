use tokio::sync::broadcast;

use crate::message::ServerPayload;

#[derive(Debug, Clone)]
pub enum ServerEvent {
    /// 发送消息给指定玩家
    SendMessageToPlayer {
        player_id: u64,
        payload: ServerPayload,
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
