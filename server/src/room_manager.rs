use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use tokio::sync::{
    RwLock,
    mpsc::{self, Sender},
};

use crate::{
    events::EventBus,
    room::{RoomActor, RoomActorMessage},
    session::SessionManager,
};

/// 全局唯一的房间ID生成器
static NEXT_ROOM_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct RoomManager {
    /// 房间ID生成器
    rooms: Arc<RwLock<HashMap<u64, Sender<RoomActorMessage>>>>,
}

impl RoomManager {
    pub fn new() -> Self {
        Self {
            rooms: Default::default(),
        }
    }

    pub async fn get_latest_room_id(&self) -> u64 {
        NEXT_ROOM_ID.load(Ordering::SeqCst) - 1
    }

    pub async fn get_room_sender(&self, room_id: u64) -> Option<Sender<RoomActorMessage>> {
        self.rooms.read().await.get(&room_id).cloned()
    }

    /// 获取当前房间数量
    pub async fn room_count(&self) -> usize {
        self.rooms.read().await.len()
    }

    pub async fn create_room(
        &self,
        players: [u64; 2],
        event_bus: EventBus,
        session_manager: SessionManager,
    ) -> u64 {
        let room_id = NEXT_ROOM_ID.fetch_add(1, Ordering::SeqCst);
        // 1. 创建 RoomActor
        let (sender, receiver) = mpsc::channel(128);
        let mut room_actor = RoomActor::new(room_id, event_bus, players, receiver, session_manager);
        // 2. 启动 RoomActor
        tokio::spawn(async move {
            room_actor.run().await;
        });
        // 3. 加入房间
        self.rooms.write().await.insert(room_id, sender);
        room_id
    }

    pub async fn remove_room(&self, room_id: u64) {
        // 获取房间的sender并发送关闭消息
        if let Some(sender) = self.rooms.write().await.remove(&room_id) {
            // 发送关闭消息给RoomActor
            if let Err(e) = sender.send(RoomActorMessage::Close).await {
                println!("发送关闭消息到房间 {} 失败: {:?}", room_id, e);
            }
            println!("房间 {} 已移除并发送关闭消息", room_id);
        } else {
            println!("房间 {} 不存在", room_id);
        }
    }
}
