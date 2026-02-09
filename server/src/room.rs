use std::{collections::HashMap, time::Duration};

use common::{
    buff_effect::ExceptionEffect,
    message::RoomAction,
    sprites::{Sprite, skills::Skill},
};
use tokio::{sync::mpsc::Receiver, time::Instant};

use crate::{
    actor::{ActorMessage, SystemMessage},
    events::EventBus,
    session::SessionManager,
};

pub enum RoomActorMessage {
    SpriteTeam {
        player_id: u64,
        sprite_team: Vec<Sprite>,
    },
    Close,
    RoomAction(RoomAction),
}

pub struct RoomActor {
    room_id: u64,
    event_bus: EventBus,
    players: [u64; 2],
    game_state: GameState,
    receiver: Receiver<RoomActorMessage>,
    session_manager: SessionManager,
}

impl RoomActor {
    pub fn new(
        room_id: u64,
        event_bus: EventBus,
        players: [u64; 2],
        receiver: Receiver<RoomActorMessage>,
        session_manager: SessionManager,
    ) -> Self {
        Self {
            room_id,
            event_bus,
            players,
            game_state: GameState::default(),
            receiver,
            session_manager,
        }
    }

    pub async fn run(&mut self) {
        println!("房间 {} 的Actor正在运行", self.room_id);
        // 每秒检查一次
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        while !self.game_state.is_end() {
            tokio::select! {
                _ = interval.tick() => {
                    if self.game_state.pk_state.is_timeout() {
                        println!("[Room {}] Deadline reached.", self.room_id);
                        self.handle_timeout().await;
                    }
                },
                Some(msg) = self.receiver.recv() => {
                    match msg {
                        RoomActorMessage::SpriteTeam {
                            player_id,
                            sprite_team,
                        } if self.players.contains(&player_id) => {
                            println!("[RoomActor {}] 玩家 {} 提交了", self.room_id, player_id);
                            // 1. 初始化玩家精灵队伍
                            self.game_state.sprite_teams.insert(player_id, sprite_team);
                            // 2. 设置精灵队伍的首精灵为当前上场精灵
                            self.game_state.current_sprite_players.insert(player_id, 0);
                            // 3. 等待双方精灵数据提交完成
                            if self.game_state.is_teams_ready() {
                                // 4. 通知双方精灵队伍已准备好，选择使用的技能TODO:
                                // 5. 进入等待技能释放状态
                                self.game_state.pk_state.start_waiting_skill();
                            }
                        }
                        RoomActorMessage::RoomAction(room_action) => {
                            self.handle_room_action(room_action).await;
                        }
                        RoomActorMessage::Close => {
                            println!("房间 {} 的Actor正在关闭", self.room_id);
                            break; // 退出循环，关闭Actor
                        }
                        _ => {}
                    }
                }
            }
        }
        println!("房间 {} 的Actor已关闭", self.room_id);
    }

    async fn handle_room_action(&mut self, room_action: RoomAction) -> anyhow::Result<()> {
        match room_action {
            RoomAction::SkillAttack {
                player_id,
                skill_id,
            } => {
                println!(
                    "[RoomActor {}] 玩家 {} 攻击了技能 {:?}",
                    self.room_id, player_id, skill_id
                );
                // 1. 记录玩家提交的操作
                self.game_state.room_actions.insert(player_id, room_action);
                // 2. 检查是否所有玩家都提交了操作
                if self
                    .game_state
                    .is_room_actions_ready(self.get_target_player_id(player_id))
                {
                    // 3. 处理玩家提交的操作
                    self.handle_room_actions();
                }
            }
            RoomAction::SwitchSprite {
                player_id,
                sprite_index,
            } => {
                // 1. 切换上场的精灵
                self.game_state
                    .switch_current_sprite(player_id, sprite_index)?;
                // 2. 记录玩家提交的操作
                self.game_state.room_actions.insert(player_id, room_action);
                // 2. 检查是否所有玩家都提交了操作
                if self
                    .game_state
                    .is_room_actions_ready(self.get_target_player_id(player_id))
                {
                    // 3. 处理玩家提交的操作
                    self.handle_room_actions();
                }
            }
            RoomAction::UseItem { player_id, .. } => {
                // 1. 记录玩家提交的物品
                self.game_state.room_actions.insert(player_id, room_action);
                // 2. 检查是否所有玩家都提交了操作
                if self
                    .game_state
                    .is_room_actions_ready(self.get_target_player_id(player_id))
                {
                    // 3. 处理玩家提交的操作
                    self.handle_room_actions();
                }
            }
            RoomAction::CatchSprite => {
                // 1. 判断是否可以捕获精灵,
                if self.game_state.is_catch_sprite_ready() {
                    // 2. 记录玩家提交的操作
                }
            }
            RoomAction::Escape(player_id) => {
                // 通知对方玩家逃跑了
                self.session_manager
                    .send_message(
                        self.get_target_player_id(player_id),
                        ActorMessage::SystemNotification(SystemMessage::Escape(player_id)),
                    )
                    .await;
                // 结束战斗，关闭房间
                self.game_state.pk_state.end_battle();
            }
        }
        Ok(())
    }

    fn handle_room_actions(&mut self) {}

    async fn handle_skill_attack(&mut self, player_id: u64, skill_id: u64) {
        // 1. 记录玩家提交的技能
        // self.game_state.skill_players.insert(player_id, skill_id);
        // // 需要等待双方都提交了技能，还要处理被控制无法攻击的情况，不用等待技能
        // let other_player_id = if self.players[0] == player_id {
        //     self.players[1]
        // } else {
        //     self.players[0]
        // };
        // let mut is_await = true;
        // if let Some(exception_effect) = self.game_state.exception_effects.get(&player_id)
        //     && matches!(exception_effect, ExceptionEffect::Numbness)
        //     && matches!(exception_effect, ExceptionEffect::Freeze)
        // {
        //     is_await = false;
        // }
        // if is_await {
        //     if self.game_state.is_skill_ready() {
        //         let Some(battle_sprite_a) = self.sprite_and_skill(0) else {
        //             return;
        //         };
        //         let Some(battle_sprite_b) = self.sprite_and_skill(1) else {
        //             return;
        //         };
        //     }
        // } else {
        //     // 不需要等待对方提交技能，直接处理
        // }
    }

    fn sprite_and_skill(&mut self, index: usize) -> Option<RoomAction> {
        // 1. 获取双方玩家的id
        let player_id = self.players[index];
        // 2. 获取当前上场精灵
        let Some(current_sprite) = self.current_sprite(player_id) else {
            return None;
        };
        // 是否需要判断该技能是玩家宠物配置的几个技能中的一个TODO:
        // 4. 获取玩家提交的操作
        let Some(room_action) = self.game_state.room_actions.remove(&player_id) else {
            return None;
        };
        // let Some(skill) = current_sprite.skills.iter().find(|s| s.id == *skill_id) else {
        //     return None;
        // };
        // // 5. 获取玩家提交的技能
        // Some(BattleSprite {
        //     sprite: current_sprite.clone(),
        //     skill: skill.clone(),
        // })
        Some(room_action)
    }

    fn current_sprite(&self, player_id: u64) -> Option<&Sprite> {
        let Some(sprite_team) = self.game_state.sprite_teams.get(&player_id) else {
            return None;
        };
        let Some(current_sprite_index) = self.game_state.current_sprite_players.get(&player_id)
        else {
            return None;
        };
        Some(&sprite_team[*current_sprite_index])
    }

    /// 处理超时
    async fn handle_timeout(&mut self) {
        match self.game_state.pk_state {
            PKState::WaitingSpriteTeams { .. } => {
                // 超时处理：结束战斗, 并通知双方TODO:
                self.game_state.pk_state = PKState::Ended;
            }
            PKState::WaitingSkillAttack { .. } => {
                // 超时处理：TODO:释放默认技能，进入下一个回合
                self.game_state.pk_state.next_turn();
            }
            _ => {}
        }
    }

    fn get_target_player_id(&self, player_id: u64) -> u64 {
        if self.players[0] == player_id {
            self.players[1]
        } else {
            self.players[0]
        }
    }
}

#[derive(Debug)]
pub struct BattleSprite {
    sprite: Sprite,
    skill: Skill,
}

#[derive(Debug, Default)]
pub struct GameState {
    /// 玩家精灵队伍
    pub sprite_teams: HashMap<u64, Vec<Sprite>>,
    /// 异常效果
    pub exception_effects: HashMap<u64, ExceptionEffect>,
    /// 玩家行为
    pub room_actions: HashMap<u64, RoomAction>,
    /// 当前上场精灵索引
    pub current_sprite_players: HashMap<u64, usize>,
    /// 战斗状态
    pub pk_state: PKState,
}

impl GameState {
    /// 判断双方是否都提交了精灵队伍
    fn is_teams_ready(&mut self) -> bool {
        self.sprite_teams.len() == 2
    }

    /// 判断双方是否都提交了行为
    fn is_room_actions_ready(&mut self, another_player_id: u64) -> bool {
        // 当对方被控制无法操作时, 则无需等待另一个玩家
        if let Some(exception_effect) = self.exception_effects.get(&another_player_id)
            && matches!(exception_effect, ExceptionEffect::Numbness)
            && matches!(exception_effect, ExceptionEffect::Freeze)
        {
            return true;
        }
        // 没有被控制, 则需要等待另一个玩家提交行为
        self.room_actions.len() == 2
    }

    /// 判断双方是否都提交了行为 TODO:
    /// 只有 `PvE` 时才有可能能够捕获精灵
    fn is_catch_sprite_ready(&mut self) -> bool {
        false
    }

    fn is_end(&self) -> bool {
        matches!(self.pk_state, PKState::Ended)
    }

    fn switch_current_sprite(&mut self, player_id: u64, sprite_index: usize) -> anyhow::Result<()> {
        // 检查目标精灵的HP值是否大于0
        let Some(target_sprite) = self
            .sprite_teams
            .get(&player_id)
            .map(|team| &team[sprite_index])
        else {
            return Err(anyhow::anyhow!("目标精灵不存在"));
        };
        if target_sprite.hp <= 0 {
            return Err(anyhow::anyhow!("目标精灵生命值小于等于0"));
        }
        // 切换上场的精灵
        self.current_sprite_players.insert(player_id, sprite_index);
        Ok(())
    }
}

#[derive(Debug, Default)]
pub enum PKState {
    /// 战斗开始
    #[default]
    Start,
    /// 等待获取玩家队伍数据
    WaitingSpriteTeams { deadline: Instant },
    /// 等待玩家释放技能
    WaitingSkillAttack { turn_deadline: Instant },

    /// 战斗结束
    Ended,
}

impl PKState {
    /// 从战斗开始状态开始等待玩家队伍数据
    fn start_waiting_teams(&mut self) {
        if let PKState::Start = self {
            *self = PKState::WaitingSpriteTeams {
                deadline: Instant::now() + std::time::Duration::from_secs(60),
            };
        }
    }

    /// 从等待精灵队伍状态进入等待技能释放状态
    fn start_waiting_skill(&mut self) {
        if matches!(self, PKState::WaitingSpriteTeams { .. }) {
            *self = PKState::WaitingSkillAttack {
                turn_deadline: Instant::now() + std::time::Duration::from_secs(10),
            };
        }
    }

    /// 检查是否超时
    fn is_timeout(&self) -> bool {
        match self {
            PKState::WaitingSpriteTeams { deadline } => Instant::now() > *deadline,
            PKState::WaitingSkillAttack { turn_deadline } => Instant::now() > *turn_deadline,
            _ => false,
        }
    }

    /// 进入下一个回合
    fn next_turn(&mut self) {
        if matches!(self, PKState::WaitingSkillAttack { .. }) {
            *self = PKState::WaitingSkillAttack {
                turn_deadline: Instant::now() + std::time::Duration::from_secs(10),
            };
        }
    }

    /// 结束战斗
    fn end_battle(&mut self) {
        *self = PKState::Ended;
    }
}
