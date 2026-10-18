//! 持有物理串口的本地回环代理。

use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

use miresidian::MiresidianResult;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};

use super::protocol::{self, ACQUIRE, CLEAR, CONTROL, QUERY, READ, RELEASE, STATUS, WRITE};
use crate::transport::{
    BufferDirection, ControlSignalState, ReceiveMode, ReceiveOptions, SerialSettings, io_failure,
    serial_failure,
};

static LOCAL_SERVER: OnceLock<()> = OnceLock::new();
static NEXT_LEASE: AtomicU64 = AtomicU64::new(1);

/// 启动当前 DLL 承载的代理线程。多次调用不会重复启动。
pub(super) fn start(listener: TcpListener) -> MiresidianResult<()> {
    pin_broker_module()?;
    LOCAL_SERVER.get_or_init(|| {
        let registry = Arc::new(BrokerRegistry::default());
        thread::Builder::new()
            .name("miresidian-serial-broker".to_owned())
            .spawn(move || serve(listener, registry))
            .expect("无法启动 Miresidian 串口代理线程");
    });
    Ok(())
}

/// 代理线程的代码位于首个驱动 DLL 中；Windows 上增加模块引用，避免宿主过早卸载该 DLL。
#[cfg(windows)]
fn pin_broker_module() -> MiresidianResult<()> {
    unsafe extern "system" {
        fn GetModuleHandleExW(
            flags: u32,
            module_name: *const u16,
            module: *mut *mut core::ffi::c_void,
        ) -> i32;
    }

    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
    let mut module = core::ptr::null_mut();
    // SAFETY: `FROM_ADDRESS` 模式要求第二个参数是当前模块内有效函数地址；
    // `pin_broker_module` 满足该条件，`module` 是可写的局部输出指针。
    let succeeded = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
            pin_broker_module as *const () as *const u16,
            &mut module,
        )
    };
    if succeeded == 0 || module.is_null() {
        return Err(protocol::failure("无法固定串口代理宿主 DLL"));
    }
    Ok(())
}

/// 非 Windows 平台上的动态库生命周期由宿主负责；实现保持本地测试可用。
#[cfg(not(windows))]
fn pin_broker_module() -> MiresidianResult<()> {
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
    while let Ok((operation, payload)) = protocol::read_frame(&mut stream) {
        let response = handle_request(&registry, operation, &payload, &mut owned_leases);
        if protocol::write_response(&mut stream, response).is_err() {
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
) -> MiresidianResult<Vec<u8>> {
    match operation {
        ACQUIRE => {
            let lease = registry.acquire(protocol::decode_settings(payload)?)?;
            owned_leases.push(lease);
            Ok(lease.to_vec())
        }
        WRITE => {
            let (lease, bytes) = protocol::decode_lease_bytes(payload)?;
            registry.write(lease, bytes)?;
            Ok(Vec::new())
        }
        READ => {
            let (lease, options) = protocol::decode_read(payload)?;
            registry.read(lease, options)
        }
        QUERY => {
            let (lease, bytes, options) = protocol::decode_query_with_options(payload)?;
            registry.query(lease, bytes, options)
        }
        CLEAR => {
            let (lease, direction) = protocol::decode_clear(payload)?;
            registry.clear(lease, direction)?;
            Ok(Vec::new())
        }
        CONTROL => {
            let (lease, dtr, rts, break_state) = protocol::decode_control(payload)?;
            registry.control(lease, dtr, rts, break_state)?;
            Ok(Vec::new())
        }
        STATUS => {
            let lease = protocol::decode_lease(payload)?;
            let (read, write) = registry.status(lease)?;
            let mut out = Vec::new();
            out.extend_from_slice(
                &(u64::try_from(read).map_err(|_| protocol::failure("待读字节数超出范围"))?)
                    .to_be_bytes(),
            );
            out.extend_from_slice(
                &(u64::try_from(write).map_err(|_| protocol::failure("待写字节数超出范围"))?)
                    .to_be_bytes(),
            );
            Ok(out)
        }
        RELEASE => {
            let lease = protocol::decode_lease(payload)?;
            registry.release(lease);
            owned_leases.retain(|current| current != &lease);
            Ok(Vec::new())
        }
        _ => Err(protocol::failure("串口代理收到不支持的请求")),
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
    read_timeout_ms: u64,
    write_timeout_ms: u64,
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
    fn acquire(&self, settings: SerialSettings) -> MiresidianResult<[u8; 16]> {
        let key = settings.port_name.trim().to_uppercase();
        let physical = PhysicalSettings::from(&settings);
        let mut state = self
            .state
            .lock()
            .map_err(|_| protocol::failure("串口代理状态锁已损坏"))?;
        let entry = if let Some(entry) = state.ports.get(&key) {
            if entry.settings != physical {
                return Err(protocol::failure(format!(
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
                .timeout(Duration::from_millis(settings.read_timeout_ms))
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
                read_timeout_ms: settings.read_timeout_ms,
                write_timeout_ms: settings.write_timeout_ms,
            },
        );
        let _ = entry;
        Ok(lease)
    }

    fn write(&self, lease: [u8; 16], bytes: Vec<u8>) -> MiresidianResult<()> {
        self.with_port(lease, true, |port| {
            port.write_all(&bytes)
                .map_err(|error| io_failure("写入", error))?;
            port.flush().map_err(|error| io_failure("刷新", error))
        })
    }

    fn read(&self, lease: [u8; 16], options: ReceiveOptions) -> MiresidianResult<Vec<u8>> {
        self.with_port_timeout(lease, options.timeout_ms, |port| read_until(port, &options))
    }

    fn query(
        &self,
        lease: [u8; 16],
        bytes: Vec<u8>,
        options: ReceiveOptions,
    ) -> MiresidianResult<Vec<u8>> {
        self.with_port(lease, true, |port| {
            port.write_all(&bytes)
                .map_err(|error| io_failure("写入", error))?;
            port.flush().map_err(|error| io_failure("刷新", error))?;
            read_until(port, &options)
        })
    }

    fn with_port<T>(
        &self,
        lease: [u8; 16],
        write: bool,
        action: impl FnOnce(&mut dyn SerialPort) -> MiresidianResult<T>,
    ) -> MiresidianResult<T> {
        self.with_port_timeout(
            lease,
            if write {
                self.entry_for(lease)?.1.write_timeout_ms
            } else {
                0
            },
            action,
        )
    }
    fn with_port_timeout<T>(
        &self,
        lease: [u8; 16],
        timeout_ms: u64,
        action: impl FnOnce(&mut dyn SerialPort) -> MiresidianResult<T>,
    ) -> MiresidianResult<T> {
        let (entry, info) = self.entry_for(lease)?;
        let timeout_ms = if timeout_ms == 0 {
            info.read_timeout_ms
        } else {
            timeout_ms
        };
        let mut port = entry
            .port
            .lock()
            .map_err(|_| protocol::failure("串口通信锁已损坏"))?;
        port.set_timeout(Duration::from_millis(timeout_ms))
            .map_err(|error| io_failure("设置超时", error.into()))?;
        action(port.as_mut())
    }

    fn entry_for(&self, lease: [u8; 16]) -> MiresidianResult<(Arc<PortEntry>, LeaseInfo)> {
        let state = self
            .state
            .lock()
            .map_err(|_| protocol::failure("串口代理状态锁已损坏"))?;
        let info = state
            .leases
            .get(&lease)
            .ok_or_else(|| protocol::failure("串口租约已失效"))?;
        let entry = state
            .ports
            .get(&info.port_key)
            .ok_or_else(|| protocol::failure("串口租约对应端口不存在"))?;
        Ok((
            Arc::clone(entry),
            LeaseInfo {
                port_key: info.port_key.clone(),
                read_timeout_ms: info.read_timeout_ms,
                write_timeout_ms: info.write_timeout_ms,
            },
        ))
    }

    fn clear(&self, lease: [u8; 16], direction: BufferDirection) -> MiresidianResult<()> {
        self.with_port(lease, false, |port| {
            port.clear(match direction {
                BufferDirection::Input => ClearBuffer::Input,
                BufferDirection::Output => ClearBuffer::Output,
                BufferDirection::Both => ClearBuffer::All,
            })
            .map_err(|error| io_failure("清空缓冲区", error.into()))
        })
    }
    fn control(
        &self,
        lease: [u8; 16],
        dtr: ControlSignalState,
        rts: ControlSignalState,
        break_state: ControlSignalState,
    ) -> MiresidianResult<()> {
        self.with_port(lease, false, |port| {
            if let Some(value) = signal_value(dtr) {
                port.write_data_terminal_ready(value)
                    .map_err(|e| io_failure("设置 DTR", e.into()))?;
            }
            if let Some(value) = signal_value(rts) {
                port.write_request_to_send(value)
                    .map_err(|e| io_failure("设置 RTS", e.into()))?;
            }
            if let Some(value) = signal_value(break_state) {
                port.set_break()
                    .map_err(|e| io_failure("设置中断", e.into()))?;
                if !value {
                    port.clear_break()
                        .map_err(|e| io_failure("清除中断", e.into()))?;
                }
            }
            Ok(())
        })
    }
    fn status(&self, lease: [u8; 16]) -> MiresidianResult<(usize, usize)> {
        self.with_port(lease, false, |port| {
            Ok((
                port.bytes_to_read()
                    .map_err(|e| io_failure("读取缓冲区状态", e.into()))? as usize,
                port.bytes_to_write()
                    .map_err(|e| io_failure("读取缓冲区状态", e.into()))? as usize,
            ))
        })
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

fn signal_value(value: ControlSignalState) -> Option<bool> {
    match value {
        ControlSignalState::Keep => None,
        ControlSignalState::Enabled => Some(true),
        ControlSignalState::Disabled => Some(false),
    }
}
fn read_until(port: &mut dyn SerialPort, options: &ReceiveOptions) -> MiresidianResult<Vec<u8>> {
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match port.read(&mut buffer) {
            Ok(length) => {
                received.extend_from_slice(&buffer[..length]);
                if matches!(options.mode, ReceiveMode::Length)
                    && received.len() >= options.byte_count
                {
                    received.truncate(options.byte_count);
                    return Ok(received);
                }
                if matches!(options.mode, ReceiveMode::Terminator)
                    && received.ends_with(&options.terminator)
                {
                    return Ok(received);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut && !received.is_empty() => {
                return Ok(received);
            }
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                return Err(io_failure("读取", error));
            }
            Err(error) => return Err(io_failure("读取", error)),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 未知租约无法访问代理状态() {
        let registry = BrokerRegistry::default();
        assert!(registry.write([0; 16], Vec::new()).is_err());
        assert!(
            registry
                .read(
                    [0; 16],
                    ReceiveOptions::new(ReceiveMode::IdleTimeout, Vec::new(), 0, 0)
                        .expect("空闲超时模式允许空参数"),
                )
                .is_err()
        );
        registry.release([0; 16]);
    }

    #[test]
    fn 未知协议操作不会触碰物理串口() {
        let registry = BrokerRegistry::default();
        let mut leases = Vec::new();
        assert!(handle_request(&registry, 255, &[], &mut leases).is_err());
        assert!(leases.is_empty());
    }

    #[test]
    fn 本地租约标识在连续分配时不同() {
        assert_ne!(next_lease(), next_lease());
    }
}
