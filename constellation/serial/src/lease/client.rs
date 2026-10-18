//! 运行在每个驱动 DLL 中的串口租约客户端。

use std::{
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use miresidian::MiresidianResult;

use super::{
    broker,
    protocol::{self, ACQUIRE, CLEAR, CONTROL, QUERY, READ, RELEASE, STATUS, WRITE},
};
use crate::transport::{BufferDirection, ControlSignalState, ReceiveOptions, SerialSettings};

/// 回环端点是同一主机内所有 Miresidian 串口 DLL 的固定会合点。
///
/// 只监听 `127.0.0.1`，不会暴露到网络。协议变更时必须更换端口或增加握手。
const BROKER_ADDRESS: &str = "127.0.0.1:41379";

/// 一个由本地代理持有物理串口的客户端租约。
pub(crate) struct SerialLease {
    stream: TcpStream,
    id: [u8; 16],
}

impl SerialLease {
    /// 取得与指定串口设置对应的独立调用方租约。
    pub(crate) fn acquire(settings: &SerialSettings) -> MiresidianResult<Self> {
        let mut stream = connect_broker()?;
        let response =
            protocol::request(&mut stream, ACQUIRE, &protocol::encode_settings(settings))?;
        let id = response
            .as_slice()
            .try_into()
            .map_err(|_| protocol::failure("代理返回了无效租约标识"))?;
        Ok(Self { stream, id })
    }

    /// 在当前租约对应的串口上写入并刷新数据。
    pub(crate) fn write(&mut self, bytes: &[u8]) -> MiresidianResult<()> {
        let mut payload = self.id.to_vec();
        protocol::push_bytes(&mut payload, bytes)?;
        protocol::request(&mut self.stream, WRITE, &payload).map(|_| ())
    }

    /// 从当前租约对应的串口读取数据，直到本次读取超时。
    pub(crate) fn read(&mut self, options: ReceiveOptions) -> MiresidianResult<Vec<u8>> {
        protocol::request(
            &mut self.stream,
            READ,
            &protocol::encode_read(&self.id, &options)?,
        )
    }

    /// 在同一串口锁中完成写入、刷新和读取。
    pub(crate) fn query(
        &mut self,
        bytes: &[u8],
        options: ReceiveOptions,
    ) -> MiresidianResult<Vec<u8>> {
        protocol::request(
            &mut self.stream,
            QUERY,
            &protocol::encode_query(&self.id, bytes, &options)?,
        )
    }

    pub(crate) fn clear(&mut self, direction: BufferDirection) -> MiresidianResult<()> {
        protocol::request(
            &mut self.stream,
            CLEAR,
            &protocol::encode_clear(&self.id, direction),
        )
        .map(|_| ())
    }

    pub(crate) fn set_control(
        &mut self,
        dtr: ControlSignalState,
        rts: ControlSignalState,
        break_state: ControlSignalState,
    ) -> MiresidianResult<()> {
        protocol::request(
            &mut self.stream,
            CONTROL,
            &protocol::encode_control(&self.id, dtr, rts, break_state),
        )
        .map(|_| ())
    }

    pub(crate) fn buffered_bytes(&mut self) -> MiresidianResult<(usize, usize)> {
        protocol::decode_status(&protocol::request(&mut self.stream, STATUS, &self.id)?)
    }

    fn release(&mut self) {
        let _ = protocol::request(&mut self.stream, RELEASE, &self.id);
    }
}

impl Drop for SerialLease {
    fn drop(&mut self) {
        self.release();
    }
}

/// 连接已有代理；端点尚不存在时，当前 DLL 成为代理宿主。
fn connect_broker() -> MiresidianResult<TcpStream> {
    if let Ok(stream) = TcpStream::connect(BROKER_ADDRESS) {
        return Ok(stream);
    }

    if let Ok(listener) = TcpListener::bind(BROKER_ADDRESS) {
        broker::start(listener)?;
    }

    for _ in 0..50 {
        if let Ok(stream) = TcpStream::connect(BROKER_ADDRESS) {
            return Ok(stream);
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(protocol::failure("无法连接 Miresidian 串口代理"))
}
