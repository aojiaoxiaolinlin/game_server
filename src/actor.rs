use tokio::sync::mpsc;

use crate::{
    events::{EventBus, ServerEvent},
    message::{ClientAction, ClientMessage, ClientPayload, ServerPayload},
    security::validate_token,
    session::SessionManager,
    status::PlayerSession,
};

#[derive(Debug)]
pub enum ActorMessage {
    /// 来自客户端的消息
    ClientMessage(ClientMessage),
    /// 通知 Actor 断开连接
    Disconnect,
}

pub struct PlayerActor {
    session: PlayerSession,
    receiver: mpsc::Receiver<ActorMessage>,
    event_bus: EventBus,
    session_manager: SessionManager,
}

impl PlayerActor {
    pub fn new(
        player_id: u64,
        receiver: mpsc::Receiver<ActorMessage>,
        event_bus: EventBus,
        session_manager: SessionManager,
    ) -> Self {
        Self {
            session: PlayerSession::new(player_id, 0, 0),
            receiver,
            event_bus,
            session_manager,
        }
    }

    pub async fn run(&mut self) {
        println!("[PlayerActor] 启动...");
        while let Some(msg) = self.receiver.recv().await {
            match msg {
                ActorMessage::ClientMessage(client_message) => {
                    let ClientMessage { sequence, payload } = client_message;
                    // 1. 校验序列号是否合法
                    if self.check_client_sequence(sequence)
                        && let ClientPayload::Authenticated { token, action } = payload
                    {
                        // 2. 校验 token 是否合法，
                        // TODO: 无感刷新 Refresh token
                        match validate_token(&token) {
                            Ok(_claims) => self.handle_client_action(action).await,
                            Err(e) => {
                                println!(
                                    "[PlayerActor {}] 校验 token 失败: {:?}",
                                    self.session.player_id(),
                                    e
                                );
                                self.event_bus.publish(ServerEvent::SendMessageToPlayer {
                                    player_id: self.session.player_id(),
                                    payload: ServerPayload::AuthFailed,
                                });
                            }
                        }
                    }
                }
                ActorMessage::Disconnect => {
                    println!("[PlayerActor] 收到断开连接通知");
                    self.session_manager
                        .unregister(self.session.player_id())
                        .await;
                    break;
                }
            }
        }
    }

    async fn handle_client_action(&mut self, action: crate::message::ClientAction) {
        match action {
            ClientAction::Chat { content } => {
                // 处理聊天消息
                self.event_bus.publish(ServerEvent::SendMessageToPlayer {
                    player_id: self.session.player_id(),
                    payload: ServerPayload::Chat {
                        content: format!("玩家 {} 说: {}", self.session.player_id(), content),
                    },
                });
            }
            _ => {}
        }
    }

    fn check_client_sequence(&mut self, sequence: u64) -> bool {
        if sequence <= self.session.last_client_sequence() {
            println!(
                "[PlayerActor {}] 收到重复消息，序列号: {}",
                self.session.player_id(),
                sequence
            );
            false
        } else if sequence > self.session.last_client_sequence() + 1 {
            println!(
                "[PlayerActor {}] 收到异常消息，序列号: {}",
                self.session.player_id(),
                sequence
            );
            false
        } else {
            // 2. 更新会话的最后一次客户端消息序列号
            self.session.last_client_sequence_increment();
            true
        }
    }
}
