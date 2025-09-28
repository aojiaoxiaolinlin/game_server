use std::{
    io::{Error, ErrorKind},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};
use tokio_util::{
    bytes::{Buf, BufMut, BytesMut},
    codec::{Decoder, Encoder},
};

/// 客户端发往服务端的消息结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ClientMessage {
    /// 消息序列号，用于防止"重放攻击"
    pub sequence: u64,
    pub payload: ClientPayload,
}

/// 客户端请求载体
#[derive(Debug, Serialize, Deserialize)]
pub enum ClientPayload {
    /// 心跳检测，
    Ping,
    Register,
    Login {
        username: String,
        password: String,
    },
    /// 认证成功，后续请求需要携带 token
    Authenticated {
        token: String,
        action: ClientAction,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientAction {
    Chat { content: String },
    Move { x: f32, y: f32, z: f32 },
}

/// 服务端发往客户端的消息结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerMessage {
    /// 消息序列号，用于防止"重放攻击"
    pub sequence: u64,
    pub payload: ServerPayload,
}

/// 服务端响应载体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerPayload {
    /// 心跳响应
    Pong,
    Chat {
        content: String,
    },
    /// 登录成功
    LoginSuccess(String),
    /// 登录失败
    LoginFailed,
    /// 认证失败
    AuthFailed,
}

/// 游戏消息编码器/解码器
///
/// 编码器：将 `S` 类型的消息序列化为字节流
/// 解码器：从字节流中反序列化出 `R` 类型的消息
pub struct GameMessageCodec<S, R>
where
    S: Serialize,
    R: for<'de> Deserialize<'de>,
{
    _phantom: PhantomData<(S, R)>,
}

impl<S, R> Default for GameMessageCodec<S, R>
where
    S: Serialize,
    R: for<'de> Deserialize<'de>,
{
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S, R> Encoder<S> for GameMessageCodec<S, R>
where
    S: Serialize,
    R: for<'de> Deserialize<'de>,
{
    type Error = Error;

    fn encode(&mut self, item: S, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // 序列化消息
        let bytes = bincode::serde::encode_to_vec(item, bincode::config::standard())
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        // 写入消息内容长度前缀
        dst.put_u32(bytes.len() as u32);
        // 写入消息体
        dst.extend_from_slice(&bytes);
        Ok(())
    }
}

impl<S, R> Decoder for GameMessageCodec<S, R>
where
    S: Serialize,
    R: for<'de> Deserialize<'de>,
{
    type Item = R;

    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            // 长度前缀不足
            return Ok(None);
        }
        let len = {
            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&src[..4]);
            u32::from_be_bytes(len_bytes) as usize
        };
        if src.len() < 4 + len {
            // 消息体不完整
            // 这里是为了避免频繁分配内存，可以预留一些空间
            // 额外预留的值 = 预计消息体长度 + 4 - 当前缓冲区长度
            src.reserve(4 + len - src.len());
            return Ok(None);
        }

        // 跳过长度前缀
        src.advance(4);
        // 读取消息体
        let msg_bytes = src.split_to(len);
        // 反序列化消息
        let (msg, _) = bincode::serde::decode_from_slice(&msg_bytes, bincode::config::standard())
            .map_err(|e| Error::new(ErrorKind::Other, e))?;
        Ok(Some(msg))
    }
}
