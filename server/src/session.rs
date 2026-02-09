use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc};

use crate::actor::ActorMessage;

#[derive(Default, Clone)]
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<u64, mpsc::Sender<ActorMessage>>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册会话
    pub async fn register(&self, id: u64, tx: mpsc::Sender<ActorMessage>) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(id, tx);
        println!(
            "[SessionManager] 玩家 {} 连接，总共在线：{}",
            id,
            sessions.len()
        );
    }

    pub async fn unregister(&self, id: u64) {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&id);
        println!(
            "[SessionManager] 玩家 {} 断开连接，总共在线：{}",
            id,
            sessions.len()
        );
    }

    pub async fn get_session(&self, id: u64) -> Option<mpsc::Sender<ActorMessage>> {
        let sessions = self.sessions.read().await;
        sessions.get(&id).cloned()
    }

    pub async fn send_message(&self, id: u64, message: ActorMessage) {
        if let Some(session) = self.get_session(id).await {
            session.send(message).await;
        } else {
            println!("[SessionManager] 玩家 {} 不存在", id);
        }
    }
}
