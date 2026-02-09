use tokio::sync::mpsc;

use crate::{
    events::{EventBus, ServerEvent},
    room::RoomActorMessage,
    room_manager::RoomManager,
    session::SessionManager,
    status::PlayerSession,
};

use common::{
    message::{ClientAction, ClientMessage, ClientPayload, RoomAction, ServerPayload},
    security::validate_token,
    sprites::{
        Sprite,
        attributes::{Attribute, SkillType},
        skills::Skill,
    },
};

#[derive(Debug, Clone)]
pub enum ActorMessage {
    /// 来自客户端的消息
    ClientMessage(ClientMessage),
    /// 系统通知
    SystemNotification(SystemMessage),
    /// 通知 Actor 断开连接
    Disconnect,
}

#[derive(Debug, Clone)]
pub enum SystemMessage {
    /// 进入房间
    EnterRoom { room_id: u64, opponent_name: String },
    /// 对方逃跑了
    Escape(u64),
}

pub struct PlayerActor {
    session: PlayerSession,
    receiver: mpsc::Receiver<ActorMessage>,
    event_bus: EventBus,
    session_manager: SessionManager,
    room_manager: RoomManager,
}

impl PlayerActor {
    pub fn new(
        player_id: u64,
        receiver: mpsc::Receiver<ActorMessage>,
        event_bus: EventBus,
        session_manager: SessionManager,
        room_manager: RoomManager,
    ) -> Self {
        Self {
            session: PlayerSession::new(player_id, 0, 0),
            receiver,
            event_bus,
            session_manager,
            room_manager,
        }
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
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
                ActorMessage::SystemNotification(system_message) => {
                    self.handle_system_notification(system_message).await?;
                }
                ActorMessage::Disconnect => {
                    println!("[PlayerActor] 收到断开连接通知");
                    self.session_manager
                        .unregister(self.session.player_id())
                        .await;
                    // 断开，跳出run，该actor运行结束
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn handle_client_action(&mut self, action: ClientAction) {
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
            ClientAction::SpriteTeam => {
                let sprite_team = get_sprite_team(self.session.player_id()).await;
                self.event_bus.publish(ServerEvent::SendMessageToPlayer {
                    player_id: self.session.player_id(),
                    payload: ServerPayload::SpriteTeam(sprite_team),
                });
            }
            ClientAction::RoomAction(room_action) => {
                self.handle_room_action(room_action).await;
            }
            _ => {}
        }
    }

    async fn handle_room_action(&mut self, room_action: RoomAction) {
        match room_action {
            RoomAction::SkillAttack { skill_id, .. } => {
                let Some(room_id) = self.session.room_id() else {
                    return;
                };
                let Some(room_sender) = self.room_manager.get_room_sender(room_id).await else {
                    return;
                };
                // 获取该技能

                if let Err(e) = room_sender
                    .send(RoomActorMessage::RoomAction(RoomAction::SkillAttack {
                        player_id: self.session.player_id(),
                        skill_id,
                    }))
                    .await
                {
                    println!(
                        "[PlayerActor {}] 发送技能攻击消息失败: {:?}",
                        self.session.player_id(),
                        e
                    );
                }
            }
            _ => {}
        }
    }

    async fn handle_system_notification(
        &mut self,
        system_message: SystemMessage,
    ) -> anyhow::Result<()> {
        match system_message {
            SystemMessage::EnterRoom {
                room_id,
                opponent_name,
            } => {
                println!(
                    "[PlayerActor {}] 进入房间 {} 与 {}",
                    self.session.player_id(),
                    room_id,
                    opponent_name
                );
                // 3. 更新会话的房间ID
                self.session.set_room_id(room_id);
                if let Some(room_sender) = self.room_manager.get_room_sender(room_id).await {
                    let sprite_team = get_sprite_team(self.session.player_id()).await;
                    room_sender
                        .send(RoomActorMessage::SpriteTeam {
                            player_id: self.session.player_id(),
                            sprite_team,
                        })
                        .await?;
                }
            }
            SystemMessage::Escape(escaped_player_id) => {
                if escaped_player_id == self.session.player_id() {
                    // 自己逃跑了，关闭房间
                    // self.room_manager
                    // 返回消息提示用户TODO:
                }
            }
        }
        Ok(())
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

// TODO:
/// 从数据库中获取玩家的精灵队伍
async fn get_sprite_team(player_id: u64) -> Vec<Sprite> {
    // TODO:从数据库中加载
    let mut preset_team_ids = Vec::with_capacity(6);
    // 临时模拟数据
    for i in 0..6 {
        let hp = 350 + rand::random::<u8>() as u16 % 101;
        let sprite = Sprite {
            id: i,
            level: 100,
            exp: 9999,
            max_exp: 10000,
            hp,
            max_hp: hp,
            phy_atk: 350 + rand::random::<u8>() as u16 % 51,
            phy_def: 240 + rand::random::<u8>() as u16 % 51,
            mag_atk: 340 + rand::random::<u8>() as u16 % 51,
            mag_def: 230 + rand::random::<u8>() as u16 % 51,
            speed: 300 + rand::random::<u8>() as u16 % 101,

            skills: vec![
                Skill {
                    id: 1,
                    name: "普通攻击".to_string(),
                    description: "普通物理攻击".to_string(),
                    skill_type: SkillType::Physical,
                    attribute: Attribute::Huo,
                    pp: 10,
                    max_pp: 10,
                    power: 100,
                    special_effect: None,
                    is_preemptive: false,
                },
                Skill {
                    id: 2,
                    name: "普通攻击".to_string(),
                    description: "普通物理攻击".to_string(),
                    skill_type: SkillType::Physical,
                    attribute: Attribute::Huo,
                    pp: 10,
                    max_pp: 10,
                    power: 100,
                    special_effect: None,
                    is_preemptive: false,
                },
            ],
        };
        preset_team_ids.push(sprite);
    }
    preset_team_ids
}

/// 从技能配置数据库中获取技能配置
///
/// # Arguments
///
/// * `skill_id` - 技能ID
///
/// # Returns
///
/// * `Skill` - 技能配置
async fn get_skill(skill_id: u64) -> Skill {
    // TODO:从数据库中加载
    let skill = Skill {
        id: skill_id,
        name: format!("技能{}", skill_id),
        description: format!("这是技能{}的描述", skill_id),
        skill_type: SkillType::Physical,
        attribute: Attribute::Huo,
        pp: 10,
        max_pp: 10,
        power: 100,
        special_effect: None,
        is_preemptive: false,
    };
    skill
}
