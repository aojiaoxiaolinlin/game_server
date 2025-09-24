use std::{net::SocketAddr, time::Duration};

use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use tcp_server::{
    actor::{ActorMessage, PlayerActor},
    events::{EventBus, ServerEvent},
    message::{ClientMessage, ClientPayload, GameMessageCodec, ServerMessage, ServerPayload},
    security::genenrate_token,
    session::SessionManager,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
    time::sleep,
};
use tokio_util::codec::Framed;

/// player_id: 使用自增Key 或者 雪花算法[`snowflake`](https://github.com/BinChengZhao/snowflake-rs)生成的唯一ID

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:5555").await.unwrap();
    let session_manager = SessionManager::new();
    let event_bus = EventBus::new();
    println!("服务器启动成功，等待客户端连接...");

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        let session_manager = session_manager.clone();
        let event_bus = event_bus.clone();
        tokio::spawn(async move {
            // 使用解码器包装 socket
            let framed = Framed::new(
                socket,
                GameMessageCodec::<ServerMessage, ClientMessage>::default(),
            );
            if let Err(e) = process(framed, addr, session_manager, event_bus).await {
                println!("处理连接 {} 时出错: {:?}", addr, e)
            };
        });
    }
}

async fn process(
    framed: Framed<TcpStream, GameMessageCodec<ServerMessage, ClientMessage>>,
    addr: SocketAddr,
    session_manager: SessionManager,
    event_bus: EventBus,
) -> anyhow::Result<()> {
    println!("接收到来自: {}的连接", addr);

    let (mut sink, mut stream) = framed.split();

    // 处理注册登录TODO:
    let player_id = authenticate(&mut sink, &mut stream).await?;

    let (actor_sender, actor_receiver) = mpsc::channel(128);
    let mut actor = PlayerActor::new(
        player_id,
        actor_receiver,
        event_bus.clone(),
        session_manager.clone(),
    );
    session_manager
        .register(player_id, actor_sender.clone())
        .await;

    // 启动Actor
    tokio::spawn(async move {
        actor.run().await;
    });

    // 订阅事件
    let mut event_subscriber = event_bus.subscribe();

    // 接收客户端的消息与Actor通信
    loop {
        tokio::select! {
            frame = stream.next()=>{
                match frame {
                    Some(Ok(frame)) => {
                        actor_sender.send(tcp_server::actor::ActorMessage::ClientMessage(frame)).await?;
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
                        match server_event {
                            ServerEvent::SendMessageToPlayer { player_id:target_player_id, payload } => {
                                if player_id == target_player_id {
                                    let msg = ServerMessage {
                                        sequence: 0,
                                        payload,
                                    };
                                    sink.send(msg).await?;
                                }
                            }
                        }
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
