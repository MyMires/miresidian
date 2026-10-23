# Miresidian Lease

`miresidian_lease` 定义设备类型无关的资源租约协议：资源键、配置一致性、随机租约 ID、获取、验证和释放。

协议不包含串口、TCP、USB、CAN 或其他设备字段；设备节点自行把其规范化配置编码为不透明字节，并在“首个租约”和“最后一个租约”事件中打开或关闭物理资源。

启用 `local-ipc` feature 时，crate 还提供同步本地 IPC 传输：Windows 使用命名管道，Unix 使用 Unix domain socket。

本 crate 不替代部署侧授权。节点必须限制请求大小、设置超时，并针对部署模型配置 Windows 管道 DACL 与服务端身份验证。
