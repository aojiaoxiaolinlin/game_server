use common::message::{ClientMessage, ClientPayload, GameMessageCodec, ServerMessage};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let stream = TcpStream::connect("127.0.0.1:5555").await?;

    println!("连接到服务器: {:?}", stream.peer_addr().unwrap());

    let mut framed = Framed::new(
        stream,
        GameMessageCodec::<ClientMessage, ServerMessage>::default(),
    );

    // 向服务端发送数据
    let msg = ClientPayload::Login {
        username: "account".to_owned(),
        password: "password".to_owned(),
    };
    let msg = ClientMessage {
        sequence: 1,
        payload: msg,
    };

    framed.send(msg).await?;

    // 接收服务端响应
    while let Some(msg) = framed.next().await {
        match msg {
            Ok(msg) => {
                println!("收到服务端响应: {:?}", msg);
            }
            Err(e) => {
                println!("接收服务端响应出错: {:?}", e);
            }
        }
    }

    // 关闭连接
    framed.close().await?;

    Ok(())
}
