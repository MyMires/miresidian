# Miresidian Lease

`miresidian_lease` 定义设备类型无关的资源租约协议：资源键、配置一致性、随机租约 ID、获取、验证和释放。

协议不包含串口、TCP、USB、CAN 或其他设备字段；节点宏把其规范化配置编码为不透明字节，业务节点不直接接触租约令牌。

启用 `local-ipc` feature 时，首个调用方创建本地租约服务，其他 DLL 通过同步 IPC 连接到同一服务：Windows 使用命名管道，Unix 使用 Unix domain socket。兼容配置可取得共享租约；连接关闭或显式释放时服务端回收令牌。

本 crate 不替代部署侧授权。节点必须限制请求大小、设置超时，并针对部署模型配置 Windows 管道 DACL 与服务端身份验证。
