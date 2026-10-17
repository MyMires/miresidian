//! 跨 DLL 串口租约客户端与嵌入式本地代理。
//!
//! 首个调用 `InitDevice` 的 DLL 绑定固定回环端口并在后台持有端口池；其他目录中的
//! 同类 DLL 通过 TCP 回环连接该代理。测试平台始终只调用驱动的 `invoke` 入口。

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};

use crate::{DriverError, DriverResult};

use super::{SerialSettings, io_failure, serial_failure};

/// 回环端点是进程内所有 Miresidian 驱动的固定会合点。
///
/// 只监听 `127.0.0.1`，不会暴露到网络。协议版本变更时必须更换端口或增加握手。
const BROKER_ADDRESS: &str = "127.0.0.1:41379";
const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;
const ACQUIRE: u8 = 1;
const WRITE: u8 = 2;
const READ: u8 = 3;
const QUERY: u8 = 4;
const RELEASE: u8 = 5;

static LOCAL_SERVER: OnceLock<()> = OnceLock::new();
static NEXT_LEASE: AtomicU64 = AtomicU64::new(1);

/// 一个由代理持有物理串口的客户端租约。
pub(super) struct SerialLease {
    stream: TcpStream,
    id: [u8; 16],
}

impl SerialLease {
    pub(super) fn acquire(settings: &SerialSettings) -> DriverResult<Self> {
        let mut stream = connect_broker()?;
        let response = request(&mut stream, ACQUIRE, &encode_settings(settings))?;
        let id: [u8; 16] = response
            .as_slice()
            .try_into()
            .map_err(|_| failure("代理返回了无效租约标识"))?;
        Ok(Self { stream, id })
    }

    pub(super) fn write(&mut self, bytes: &[u8]) -> DriverResult<()> {
        let mut payload = self.id.to_vec();
        push_bytes(&mut payload, bytes)?;
        request(&mut self.stream, WRITE, &payload).map(|_| ())
    }

    pub(super) fn read(&mut self, maximum_bytes: usize) -> DriverResult<Vec<u8>> {
        let maximum_bytes =
            u32::try_from(maximum_bytes).map_err(|_| failure("最大接收字节数超出代理支持范围"))?;
        let mut payload = self.id.to_vec();
        payload.extend_from_slice(&maximum_bytes.to_be_bytes());
        request(&mut self.stream, READ, &payload)
    }

    pub(super) fn query(&mut self, bytes: &[u8], maximum_bytes: usize) -> DriverResult<Vec<u8>> {
        let maximum_bytes =
            u32::try_from(maximum_bytes).map_err(|_| failure("最大接收字节数超出代理支持范围"))?;
        let mut payload = self.id.to_vec();
        push_bytes(&mut payload, bytes)?;
        payload.extend_from_slice(&maximum_bytes.to_be_bytes());
        request(&mut self.stream, QUERY, &payload)
    }

    fn release(&mut self) {
        let _ = request(&mut self.stream, RELEASE, &self.id);
    }
}

impl Drop for SerialLease {
    fn drop(&mut self) {
        self.release();
    }
}

/// 连接现有代理；端点尚不存在时当前 DLL 负责成为代理宿主。
fn connect_broker() -> DriverResult<TcpStream> {
    if let Ok(stream) = TcpStream::connect(BROKER_ADDRESS) {
        return Ok(stream);
    }

    if let Ok(listener) = TcpListener::bind(BROKER_ADDRESS) {
        pin_broker_module()?;
        LOCAL_SERVER.get_or_init(|| {
            let registry = Arc::new(BrokerRegistry::default());
            thread::Builder::new()
                .name("miresidian-serial-broker".to_owned())
                .spawn(move || serve(listener, registry))
                .expect("无法启动 Miresidian 串口代理线程");
        });
    }

    for _ in 0..50 {
        if let Ok(stream) = TcpStream::connect(BROKER_ADDRESS) {
            return Ok(stream);
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err(failure("无法连接 Miresidian 串口代理"))
}

/// 代理线程的代码位于首个驱动 DLL 中；Windows 上增加模块引用，避免宿主过早卸载该 DLL。
#[cfg(windows)]
fn pin_broker_module() -> DriverResult<()> {
    unsafe extern "system" {
        fn GetModuleHandleExW(
            flags: u32,
            module_name: *const u16,
            module: *mut *mut core::ffi::c_void,
        ) -> i32;
    }

    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
    let mut module = core::ptr::null_mut();
    // 函数地址必定位于承载当前代理线程的驱动 DLL 中。
    let succeeded = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            pin_broker_module as *const () as *const u16,
            &mut module,
        )
    };
    if succeeded == 0 || module.is_null() {
        return Err(failure("无法固定串口代理宿主 DLL"));
    }
    Ok(())
}

/// 非 Windows 平台上的动态库生命周期由宿主负责；该实现保持本地测试可用。
#[cfg(not(windows))]
fn pin_broker_module() -> DriverResult<()> {
    Ok(())
}

fn serve(listener: TcpListener, registry: Arc<BrokerRegistry>) {
    for stream in listener.incoming().flatten() {
        let registry = Arc::clone(&registry);
        let _ = thread::Builder::new()
            .name("miresidian-serial-client".to_owned())
            .spawn(move || serve_client(stream, registry));
    }
}

fn serve_client(mut stream: TcpStream, registry: Arc<BrokerRegistry>) {
    let mut owned_leases = Vec::new();
    while let Ok((operation, payload)) = read_frame(&mut stream) {
        let response = handle_request(&registry, operation, &payload, &mut owned_leases);
        if write_response(&mut stream, response).is_err() {
            break;
        }
    }
    for lease in owned_leases {
        registry.release(lease);
    }
}

fn handle_request(
    registry: &BrokerRegistry,
    operation: u8,
    payload: &[u8],
    owned_leases: &mut Vec<[u8; 16]>,
) -> DriverResult<Vec<u8>> {
    match operation {
        ACQUIRE => {
            let settings = decode_settings(payload)?;
            let lease = registry.acquire(settings)?;
            owned_leases.push(lease);
            Ok(lease.to_vec())
        }
        WRITE => {
            let (lease, bytes) = decode_lease_bytes(payload)?;
            registry.write(lease, bytes)?;
            Ok(Vec::new())
        }
        READ => {
            let (lease, maximum_bytes) = decode_lease_length(payload)?;
            registry.read(lease, maximum_bytes)
        }
        QUERY => {
            let (lease, bytes, maximum_bytes) = decode_query(payload)?;
            registry.query(lease, bytes, maximum_bytes)
        }
        RELEASE => {
            let lease = decode_lease(payload)?;
            registry.release(lease);
            owned_leases.retain(|current| current != &lease);
            Ok(Vec::new())
        }
        _ => Err(failure("串口代理收到不支持的请求")),
    }
}

#[derive(Default)]
struct BrokerRegistry {
    state: Mutex<RegistryState>,
}

#[derive(Default)]
struct RegistryState {
    ports: HashMap<String, Arc<PortEntry>>,
    leases: HashMap<[u8; 16], LeaseInfo>,
}

struct LeaseInfo {
    port_key: String,
    timeout_ms: u64,
}

struct PortEntry {
    settings: PhysicalSettings,
    port: Mutex<Box<dyn SerialPort>>,
}

#[derive(Clone, PartialEq, Eq)]
struct PhysicalSettings {
    port_name: String,
    baud_rate: u32,
    data_bits: DataBits,
    parity: Parity,
    stop_bits: StopBits,
    flow_control: FlowControl,
}

impl BrokerRegistry {
    fn acquire(&self, settings: SerialSettings) -> DriverResult<[u8; 16]> {
        let key = settings.port_name.trim().to_uppercase();
        let physical = PhysicalSettings::from(&settings);
        let mut state = self
            .state
            .lock()
            .map_err(|_| failure("串口代理状态锁已损坏"))?;
        let entry = if let Some(entry) = state.ports.get(&key) {
            if entry.settings != physical {
                return Err(failure(format!(
                    "串口 {key} 已按其他物理通信参数打开，无法共享"
                )));
            }
            Arc::clone(entry)
        } else {
            let port = serialport::new(&settings.port_name, settings.baud_rate)
                .data_bits(settings.data_bits)
                .parity(settings.parity)
                .stop_bits(settings.stop_bits)
                .flow_control(settings.flow_control)
                .timeout(Duration::from_millis(settings.timeout_ms))
                .open()
                .map_err(|error| serial_failure("打开", error))?;
            let entry = Arc::new(PortEntry {
                settings: physical,
                port: Mutex::new(port),
            });
            state.ports.insert(key.clone(), Arc::clone(&entry));
            entry
        };
        let lease = next_lease();
        state.leases.insert(
            lease,
            LeaseInfo {
                port_key: key,
                timeout_ms: settings.timeout_ms,
            },
        );
        let _ = entry;
        Ok(lease)
    }

    fn write(&self, lease: [u8; 16], bytes: Vec<u8>) -> DriverResult<()> {
        self.with_port(lease, |port| {
            port.write_all(&bytes)
                .map_err(|error| io_failure("写入", error))?;
            port.flush().map_err(|error| io_failure("刷新", error))
        })
    }

    fn read(&self, lease: [u8; 16], maximum_bytes: usize) -> DriverResult<Vec<u8>> {
        self.with_port(lease, |port| read_bytes(port, maximum_bytes))
    }

    fn query(
        &self,
        lease: [u8; 16],
        bytes: Vec<u8>,
        maximum_bytes: usize,
    ) -> DriverResult<Vec<u8>> {
        self.with_port(lease, |port| {
            port.write_all(&bytes)
                .map_err(|error| io_failure("写入", error))?;
            port.flush().map_err(|error| io_failure("刷新", error))?;
            read_bytes(port, maximum_bytes)
        })
    }

    fn with_port<T>(
        &self,
        lease: [u8; 16],
        action: impl FnOnce(&mut dyn SerialPort) -> DriverResult<T>,
    ) -> DriverResult<T> {
        let (entry, timeout_ms) = self.entry_for(lease)?;
        let mut port = entry.port.lock().map_err(|_| failure("串口通信锁已损坏"))?;
        port.set_timeout(Duration::from_millis(timeout_ms))
            .map_err(|error| io_failure("设置超时", error.into()))?;
        action(port.as_mut())
    }

    fn entry_for(&self, lease: [u8; 16]) -> DriverResult<(Arc<PortEntry>, u64)> {
        let state = self
            .state
            .lock()
            .map_err(|_| failure("串口代理状态锁已损坏"))?;
        let info = state
            .leases
            .get(&lease)
            .ok_or_else(|| failure("串口租约已失效"))?;
        let entry = state
            .ports
            .get(&info.port_key)
            .ok_or_else(|| failure("串口租约对应端口不存在"))?;
        Ok((Arc::clone(entry), info.timeout_ms))
    }

    fn release(&self, lease: [u8; 16]) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let Some(info) = state.leases.remove(&lease) else {
            return;
        };
        if !state
            .leases
            .values()
            .any(|other| other.port_key == info.port_key)
        {
            state.ports.remove(&info.port_key);
        }
    }
}

impl From<&SerialSettings> for PhysicalSettings {
    fn from(settings: &SerialSettings) -> Self {
        Self {
            port_name: settings.port_name.trim().to_uppercase(),
            baud_rate: settings.baud_rate,
            data_bits: settings.data_bits,
            parity: settings.parity,
            stop_bits: settings.stop_bits,
            flow_control: settings.flow_control,
        }
    }
}

fn read_bytes(port: &mut dyn SerialPort, maximum_bytes: usize) -> DriverResult<Vec<u8>> {
    let mut buffer = vec![0; maximum_bytes];
    let length = port
        .read(&mut buffer)
        .map_err(|error| io_failure("读取", error))?;
    buffer.truncate(length);
    Ok(buffer)
}

fn next_lease() -> [u8; 16] {
    let counter = NEXT_LEASE.fetch_add(1, Ordering::Relaxed);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut id = [0; 16];
    id[..8].copy_from_slice(&timestamp.to_be_bytes());
    id[8..].copy_from_slice(&counter.to_be_bytes());
    id
}

fn request(stream: &mut TcpStream, operation: u8, payload: &[u8]) -> DriverResult<Vec<u8>> {
    write_frame(stream, operation, payload)
        .map_err(|error| failure(format!("发送串口代理请求失败：{error}")))?;
    let (status, payload) =
        read_frame(stream).map_err(|error| failure(format!("读取串口代理响应失败：{error}")))?;
    if status == 0 {
        Ok(payload)
    } else {
        Err(failure(String::from_utf8_lossy(&payload).into_owned()))
    }
}

fn write_response(stream: &mut TcpStream, response: DriverResult<Vec<u8>>) -> std::io::Result<()> {
    match response {
        Ok(payload) => write_frame(stream, 0, &payload),
        Err(error) => write_frame(stream, 1, error.to_string().as_bytes()),
    }
}

fn write_frame(stream: &mut TcpStream, operation: u8, payload: &[u8]) -> std::io::Result<()> {
    let length = u32::try_from(payload.len()).map_err(|_| std::io::Error::other("帧过大"))?;
    stream.write_all(&[operation])?;
    stream.write_all(&length.to_be_bytes())?;
    stream.write_all(payload)?;
    stream.flush()
}

fn read_frame(stream: &mut TcpStream) -> std::io::Result<(u8, Vec<u8>)> {
    let mut operation = [0; 1];
    let mut length = [0; 4];
    stream.read_exact(&mut operation)?;
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_SIZE {
        return Err(std::io::Error::other("帧超过最大长度"));
    }
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload)?;
    Ok((operation[0], payload))
}

fn encode_settings(settings: &SerialSettings) -> Vec<u8> {
    let mut payload = Vec::new();
    push_bytes(&mut payload, settings.port_name.as_bytes()).expect("端口名称长度有效");
    payload.extend_from_slice(&settings.baud_rate.to_be_bytes());
    payload.push(data_bits_code(settings.data_bits));
    payload.push(parity_code(settings.parity));
    payload.push(stop_bits_code(settings.stop_bits));
    payload.push(flow_control_code(settings.flow_control));
    payload.extend_from_slice(&settings.timeout_ms.to_be_bytes());
    payload
}

fn decode_settings(payload: &[u8]) -> DriverResult<SerialSettings> {
    let mut offset = 0;
    let port_name = String::from_utf8(read_bytes_field(payload, &mut offset)?.to_vec())
        .map_err(|_| failure("端口名称必须为 UTF-8"))?;
    let baud_rate = read_u32(payload, &mut offset)?;
    let data_bits = decode_data_bits(read_u8(payload, &mut offset)?)?;
    let parity = decode_parity(read_u8(payload, &mut offset)?)?;
    let stop_bits = decode_stop_bits(read_u8(payload, &mut offset)?)?;
    let flow_control = decode_flow_control(read_u8(payload, &mut offset)?)?;
    let timeout_ms = read_u64(payload, &mut offset)?;
    if offset != payload.len() {
        return Err(failure("串口代理配置包含多余内容"));
    }
    Ok(SerialSettings {
        port_name,
        baud_rate,
        data_bits,
        parity,
        stop_bits,
        flow_control,
        timeout_ms,
    })
}

fn decode_lease(payload: &[u8]) -> DriverResult<[u8; 16]> {
    payload
        .try_into()
        .map_err(|_| failure("串口代理租约标识无效"))
}
fn decode_lease_bytes(payload: &[u8]) -> DriverResult<([u8; 16], Vec<u8>)> {
    let lease = payload
        .get(..16)
        .ok_or_else(|| failure("串口代理请求缺少租约"))?
        .try_into()
        .map_err(|_| failure("串口代理租约无效"))?;
    let mut offset = 16;
    Ok((lease, read_bytes_field(payload, &mut offset)?.to_vec()))
}
fn decode_lease_length(payload: &[u8]) -> DriverResult<([u8; 16], usize)> {
    let lease = payload
        .get(..16)
        .ok_or_else(|| failure("串口代理请求缺少租约"))?
        .try_into()
        .map_err(|_| failure("串口代理租约无效"))?;
    let length = payload
        .get(16..20)
        .ok_or_else(|| failure("串口代理请求缺少读取长度"))?
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| failure("串口代理读取长度无效"))?;
    Ok((lease, length as usize))
}
fn decode_query(payload: &[u8]) -> DriverResult<([u8; 16], Vec<u8>, usize)> {
    let lease = payload
        .get(..16)
        .ok_or_else(|| failure("串口代理请求缺少租约"))?
        .try_into()
        .map_err(|_| failure("串口代理租约无效"))?;
    let mut offset = 16;
    let bytes = read_bytes_field(payload, &mut offset)?.to_vec();
    let length = read_u32(payload, &mut offset)? as usize;
    Ok((lease, bytes, length))
}
fn push_bytes(target: &mut Vec<u8>, bytes: &[u8]) -> DriverResult<()> {
    let length = u32::try_from(bytes.len()).map_err(|_| failure("代理消息过长"))?;
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(bytes);
    Ok(())
}
fn read_bytes_field<'a>(payload: &'a [u8], offset: &mut usize) -> DriverResult<&'a [u8]> {
    let length = read_u32(payload, offset)? as usize;
    let end = offset
        .checked_add(length)
        .ok_or_else(|| failure("代理消息长度溢出"))?;
    let value = payload
        .get(*offset..end)
        .ok_or_else(|| failure("代理消息长度不足"))?;
    *offset = end;
    Ok(value)
}
fn read_u8(payload: &[u8], offset: &mut usize) -> DriverResult<u8> {
    let value = *payload
        .get(*offset)
        .ok_or_else(|| failure("代理消息长度不足"))?;
    *offset += 1;
    Ok(value)
}
fn read_u32(payload: &[u8], offset: &mut usize) -> DriverResult<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| failure("代理消息长度溢出"))?;
    let value = payload
        .get(*offset..end)
        .ok_or_else(|| failure("代理消息长度不足"))?
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| failure("代理消息无效"))?;
    *offset = end;
    Ok(value)
}
fn read_u64(payload: &[u8], offset: &mut usize) -> DriverResult<u64> {
    let end = offset
        .checked_add(8)
        .ok_or_else(|| failure("代理消息长度溢出"))?;
    let value = payload
        .get(*offset..end)
        .ok_or_else(|| failure("代理消息长度不足"))?
        .try_into()
        .map(u64::from_be_bytes)
        .map_err(|_| failure("代理消息无效"))?;
    *offset = end;
    Ok(value)
}
fn data_bits_code(value: DataBits) -> u8 {
    match value {
        DataBits::Five => 5,
        DataBits::Six => 6,
        DataBits::Seven => 7,
        DataBits::Eight => 8,
    }
}
fn decode_data_bits(value: u8) -> DriverResult<DataBits> {
    match value {
        5 => Ok(DataBits::Five),
        6 => Ok(DataBits::Six),
        7 => Ok(DataBits::Seven),
        8 => Ok(DataBits::Eight),
        _ => Err(failure("代理数据位无效")),
    }
}
fn parity_code(value: Parity) -> u8 {
    match value {
        Parity::None => 0,
        Parity::Odd => 1,
        Parity::Even => 2,
    }
}
fn decode_parity(value: u8) -> DriverResult<Parity> {
    match value {
        0 => Ok(Parity::None),
        1 => Ok(Parity::Odd),
        2 => Ok(Parity::Even),
        _ => Err(failure("代理校验位无效")),
    }
}
fn stop_bits_code(value: StopBits) -> u8 {
    match value {
        StopBits::One => 1,
        StopBits::Two => 2,
    }
}
fn decode_stop_bits(value: u8) -> DriverResult<StopBits> {
    match value {
        1 => Ok(StopBits::One),
        2 => Ok(StopBits::Two),
        _ => Err(failure("代理停止位无效")),
    }
}
fn flow_control_code(value: FlowControl) -> u8 {
    match value {
        FlowControl::None => 0,
        FlowControl::Software => 1,
        FlowControl::Hardware => 2,
    }
}
fn decode_flow_control(value: u8) -> DriverResult<FlowControl> {
    match value {
        0 => Ok(FlowControl::None),
        1 => Ok(FlowControl::Software),
        2 => Ok(FlowControl::Hardware),
        _ => Err(failure("代理流控无效")),
    }
}
fn failure(message: impl Into<String>) -> DriverError {
    DriverError::Failure(message.into())
}
