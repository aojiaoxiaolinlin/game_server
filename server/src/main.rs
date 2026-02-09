mod actor;
mod coordinator;
mod events;
mod game_rule;
mod matchmaking;
mod room;
mod room_manager;
mod session;
mod status;
use actor::{ActorMessage, PlayerActor};
use common::{
    message::{ClientMessage, ClientPayload, GameMessageCodec, ServerMessage, ServerPayload},
    security::genenrate_token,
};
use events::EventBus;
use events::ServerEvent;
use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use session::SessionManager;

use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    time::sleep,
};
use tokio_util::codec::Framed;

use crate::{
    coordinator::GameCoordinator, matchmaking::MatchmakingService, room_manager::RoomManager,
};

/// player_id: 使用自增Key 或者 雪花算法[`snowflake`](https://github.com/BinChengZhao/snowflake-rs)生成的唯一ID
static PLAYER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:5555").await.unwrap();
    let session_manager = SessionManager::new();
    let room_manager = RoomManager::new();
    let event_bus = EventBus::new();

    let mut matchmaking_service = MatchmakingService::new(event_bus.clone());
    let mut game_coordinator = GameCoordinator::new(
        room_manager.clone(),
        session_manager.clone(),
        event_bus.clone(),
    );

    tokio::spawn(async move {
        println!("MatchmakingService 启动成功");
        matchmaking_service.run().await;
    });
    tokio::spawn(async move {
        println!("GameCoordinator 启动成功");
        game_coordinator.run().await;
    });

    println!("服务器启动成功，等待客户端连接...");

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let session_manager = session_manager.clone();
        let room_manager = room_manager.clone();
        let event_bus = event_bus.clone();
        tokio::spawn(async move {
            // 使用解码器包装 socket
            let framed = Framed::new(
                socket,
                GameMessageCodec::<ServerMessage, ClientMessage>::default(),
            );
            if let Err(e) = process(framed, addr, session_manager, room_manager, event_bus).await {
                println!("处理连接 {} 时出错: {:?}", addr, e)
            };
        });
    }
}

async fn process(
    framed: Framed<TcpStream, GameMessageCodec<ServerMessage, ClientMessage>>,
    addr: SocketAddr,
    session_manager: SessionManager,
    room_manager: RoomManager,
    event_bus: EventBus,
) -> anyhow::Result<()> {
    println!("接收到来自: {}的连接", addr);
    // 订阅事件
    let mut event_subscriber = event_bus.subscribe();

    let (mut sink, mut stream) = framed.split();

    // 处理注册登录TODO:
    // let player_id = authenticate(&mut sink, &mut stream).await?;
    let player_id = PLAYER_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

    let (actor_sender, actor_receiver) = mpsc::channel(128);
    session_manager
        .register(player_id, actor_sender.clone())
        .await;
    let mut actor = PlayerActor::new(
        player_id,
        actor_receiver,
        event_bus,
        session_manager,
        room_manager,
    );

    // 启动Actor
    tokio::spawn(async move {
        let res = actor.run().await;
        if let Err(e) = res {
            println!("Actor 运行出错: {:?}", e);
        }
    });

    // 接收客户端的消息与Actor通信
    loop {
        tokio::select! {
            frame = stream.next()=>{
                match frame {
                    Some(Ok(frame)) => {
                        actor_sender.send(ActorMessage::ClientMessage(frame)).await?;
                    }
                    Some(Err(e)) => {
                        println!("读取消息失败: {:?}", e);
                        break;
                    }
                    None => {
                        println!("连接关闭");
                        break;
                    }
                }
            },
            event = event_subscriber.recv() =>{
                match event {
                    Ok(server_event) => {
                        handle_server_event(player_id, server_event, &mut sink).await?;
                    },
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        // 如果处理速度太慢，导致消息丢失
                        println!("用户:{} 接收事件失败: 丢失 {} 个事件", player_id, n);
                    }
                    Err(_) => break,
                }
            }
        }
    }

    // -- 断开连接 --
    actor_sender.send(ActorMessage::Disconnect).await?;
    println!("[process] 玩家 {} 断开连接", player_id);
    Ok(())
}

async fn handle_server_event(
    player_id: u64,
    server_event: ServerEvent,
    sink: &mut SplitSink<
        Framed<TcpStream, GameMessageCodec<ServerMessage, ClientMessage>>,
        ServerMessage,
    >,
) -> anyhow::Result<()> {
    match server_event {
        ServerEvent::SendMessageToPlayer {
            player_id: target_player_id,
            payload,
        } => {
            if player_id == target_player_id {
                let msg = ServerMessage {
                    sequence: 0,
                    payload,
                };
                sink.send(msg).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

async fn authenticate(
    sink: &mut SplitSink<
        Framed<TcpStream, GameMessageCodec<ServerMessage, ClientMessage>>,
        ServerMessage,
    >,
    stream: &mut SplitStream<Framed<TcpStream, GameMessageCodec<ServerMessage, ClientMessage>>>,
) -> anyhow::Result<u64> {
    let auth_timeout = sleep(Duration::from_secs(10));
    tokio::pin!(auth_timeout);

    loop {
        tokio::select! {
            frame = stream.next() => {
                match frame {
                    Some(Ok(frame)) => {
                        match frame.payload {
                            ClientPayload::Login{
                                username,
                                password,
                            } => {
                                // 验证用户名密码TOOD: 需要从数据库验证
                                if username == "account" && password =="password" {
                                    let player_id = 1;
                                    let token = genenrate_token(player_id);
                                    let response = ServerMessage {
                                        sequence: 0,
                                        payload:ServerPayload::LoginSuccess(token),
                                    };
                                    sink.send(response).await?;
                                    return Ok(player_id);
                                } else {
                                    let response = ServerMessage {
                                        sequence: 0,
                                        payload: ServerPayload::LoginFailed,
                                    };
                                    sink.send(response).await?;
                                }
                            }
                            _ => {
                                let response = ServerMessage {
                                    sequence: 0,
                                    payload: ServerPayload::LoginFailed,
                                };
                                sink.send(response).await?;
                                println!("认证失败，期望 Login 消息");
                            }
                        }
                    }
                    Some(Err(e)) => {
                        println!("读取消息失败: {:?}", e);
                        return Err(e.into());
                    }
                    None => {
                        println!("连接关闭");
                        return Err(anyhow::anyhow!("连接关闭"));
                    }
                }
            },
            _ = &mut auth_timeout => {
                println!("认证超时");
                return Err(anyhow::anyhow!("认证超时"));
            }
        }
    }
}

#[cfg(test)]
mod test {

    use tokio::time::Instant;

    use super::*;

    #[tokio::test]
    async fn test_battle() {
        let (event_bus, room_manager, session_manager) = start_server();
        let event_bus_b = event_bus.clone();

        let player_id = 1;

        let (actor_sender, actor_receiver) = mpsc::channel(128);
        let mut actor = PlayerActor::new(
            player_id,
            actor_receiver,
            event_bus.clone(),
            session_manager.clone(),
            room_manager.clone(),
        );
        session_manager
            .register(player_id, actor_sender.clone())
            .await;

        // 启动Actor
        tokio::spawn(async move {
            actor.run().await.unwrap();
        });

        let player_id = 2;

        let (actor_sender, actor_receiver) = mpsc::channel(128);
        let mut actor = PlayerActor::new(
            player_id,
            actor_receiver,
            event_bus.clone(),
            session_manager.clone(),
            room_manager.clone(),
        );
        session_manager
            .register(player_id, actor_sender.clone())
            .await;

        // 启动Actor
        tokio::spawn(async move {
            actor.run().await.unwrap();
        });

        let a = tokio::spawn(async move {
            event_bus.publish(ServerEvent::PlayerReadyForMatchmaking { player_id: 1 });
        });
        let b = tokio::spawn(async move {
            event_bus_b.publish(ServerEvent::PlayerReadyForMatchmaking { player_id: 2 });
        });
        assert!(a.await.is_ok());
        assert!(b.await.is_ok());
        let room_count = room_manager.room_count().await;
        assert_eq!(room_count, 1);
        // 等待房间创建完成
        tokio::time::sleep(Duration::from_secs(1)).await;

        let room_id = room_manager.get_latest_room_id().await;
        assert_eq!(room_id, 1);
    }

    #[tokio::test]
    async fn test_create_multiple_rooms() {
        let (event_bus, room_manager, _) = start_server();
        let mut event_bus_clones = vec![];

        // 创建6个玩家的匹配请求，应该形成3个房间
        let mut tasks = vec![];
        for i in 1..=6 {
            let event_bus_clone = event_bus.clone();
            event_bus_clones.push(event_bus_clone.clone());

            tasks.push(tokio::spawn(async move {
                event_bus_clone.publish(ServerEvent::PlayerReadyForMatchmaking { player_id: i });
            }));
        }

        // 等待所有任务完成
        for task in tasks {
            assert!(task.await.is_ok());
        }

        // 等待房间创建完成
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 验证是否创建了3个房间
        let room_count = room_manager.room_count().await;
        assert_eq!(room_count, 3, "应该创建3个房间");

        println!("成功创建3个房间");

        // 获取所有创建的房间ID并关闭它们
        let mut room_ids = vec![];
        let latest_id = room_manager.get_latest_room_id().await;

        // 假设房间ID是连续创建的
        for i in 1..=3 {
            room_ids.push(latest_id - 3 + i);
        }

        // 关闭所有房间
        for room_id in &room_ids {
            event_bus.publish(ServerEvent::CloseRoom { room_id: *room_id });
            println!("发送关闭房间 {} 的请求", room_id);
        }

        // 使用轮询方式等待所有房间关闭
        let start_time = Instant::now();
        while room_manager.room_count().await != 0 && start_time.elapsed() < Duration::from_secs(2)
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // 验证所有房间已关闭
        let room_count = room_manager.room_count().await;
        assert_eq!(room_count, 0, "所有房间应该已关闭");

        println!("成功关闭所有房间");
    }

    #[tokio::test]
    async fn test_create_room() {
        let (event_bus, room_manager, session_manager) = start_server();
        let event_bus_b = event_bus.clone();
        let event_bus_c = event_bus.clone();
        let a = tokio::spawn(async move {
            event_bus.publish(ServerEvent::PlayerReadyForMatchmaking { player_id: 1 });
        });
        let b = tokio::spawn(async move {
            event_bus_b.publish(ServerEvent::PlayerReadyForMatchmaking { player_id: 2 });
        });
        assert!(a.await.is_ok());
        assert!(b.await.is_ok());
        let room_count = room_manager.room_count().await;
        assert_eq!(room_count, 1);

        // 关闭房间
        let room_id = room_manager.get_latest_room_id().await;
        event_bus_c.publish(ServerEvent::CloseRoom { room_id });

        // 使用轮询方式等待房间关闭
        let start_time = Instant::now();
        while room_manager.room_count().await != 0 && start_time.elapsed() < Duration::from_secs(2)
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        println!("关闭房间 {} 成功", room_id);
        let room_count = room_manager.room_count().await;
        assert_eq!(room_count, 0);
    }

    fn start_server() -> (EventBus, RoomManager, SessionManager) {
        let event_bus = EventBus::new();
        let room_manager = RoomManager::new();
        let session_manager = SessionManager::new();

        let mut matchmaking_service = MatchmakingService::new(event_bus.clone());
        let mut game_coordinator = GameCoordinator::new(
            room_manager.clone(),
            session_manager.clone(),
            event_bus.clone(),
        );

        tokio::spawn(async move { matchmaking_service.run().await });
        tokio::spawn(async move { game_coordinator.run().await });

        (event_bus, room_manager, session_manager)
    }
}
