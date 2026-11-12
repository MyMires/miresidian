# 新建 Node 模板 ABI 兼容修复

## 目标、依据与授权

用户明确要求先修复 P0 模板兼容问题，授权本次工具维护及必要测试。修复 new/init 默认采用已发布最新版时因旧 ABI v7 限制失败的问题，按实际解析 ABI 生成准确的调用说明；保留精确 crates.io 依赖、已有文件保护和旧版模板能力。本任务不是交付新业务 Node，临时生成项目仅作工具回归夹具，不引入设备、可选 features 或改变框架运行时。无需额外方案确认；不推送、不发布、不改安装版本。

只读依据：Pack 0.13.5 在 template::create 写文件、生成锁并检查项目后硬拒绝非 ABI7；node.md.in 固定三参数入口。ABI 当前头文件为10，迁移说明确认7–9沿用三参数 miresidian_invoke，10改为单 JSON。registry 集成测试只固定0.14.0。CLI/套件0.3.6内嵌最低Pack版本，需同步修复后端。

## 设计与实现方式

1. 保持 Cargo 选版、精确依赖、真实锁文件和来源检查。通用模板先生成明确“ABI待解析”的调用说明，不预先声称任何调用签名；项目检查取得唯一节点实际ABI后，选择7–9旧三参数或10单JSON的说明片段，重写本次新建的说明文件，完成后才报告成功。其他ABI明确拒绝，不猜测未来契约；错误保留文件供诊断，说明仍是待完成状态。
2. 调用说明按职责拆为通用节点说明和两个ABI契约片段；旧版仅展示各版本共同支持的miresidian_invoke与版本查询，不宣称额外入口或0编号语义。v10明确platform=0/省略为默认、无旧入口、输入与返回内存所有权、status/code/output；C示例先核对ABI并立即复制响应。参数布局以GetInitConfig及实际头文件为准。
3. 模板不添加features，保留默认无硬件单实例Echo及宏生成职责。README说明按解析结果生成、失败恢复和旧项目不自动改写。
4. 为真实registry集成测试保留0.14.0旧版，增加显式当前0.33.0、ABI8/9代表版本和默认最新版new/init。覆盖生成文档、锁文件来源、编译、覆盖保护；当前版生成cdylib并通过原生动态库调用验证ABI10、初始化、Echo和关闭。原生调用验收使用临时Python/C宿主，不向框架引入新依赖。默认latest用例联网，离线显式跳过latest查询；不绕过独立Node的registry来源约束。
5. 单元测试覆盖ABI说明选择与未知ABI拒绝。CLI保留既有真实进程测试，验收统一new/init入口转发修复后的Pack。

## 影响文件与版本评估

- Pack：src/template.rs、按职责新增src/template/中的ABI文档选择模块（如有需要）、templates/node.md.in、ABI片段、README.md.in、tests/template.rs、README.md、CHANGELOG.md、Cargo.toml。
- CLI：toolset.toml、Cargo.toml、README.md、CHANGELOG.md，更新最低后端版本。
- 根Cargo.lock由Cargo更新本地工具包版本；其他锁文件仅在确有这些包记录时同步，不改独立节点依赖。根框架、宏、ABI和协议无变化，不升版。
- 版本：Pack 0.13.5→0.13.6（兼容修复既有new/init默认路径，无公开Rust API变化）；CLI/套件0.3.6→0.3.7（携带最低修复后端与文档），各归档2026-11-12 CHANGELOG。既有发行与标签不动。

## 步骤与验收

1. 落盘本方案；核实官方已发布框架、精确文档和ABI8/9代表版本。
2. 先改模板生成与ABI说明，再补回归及准确文档、独立版本和锁文件。
3. 定向cargo fmt、Pack/CLI单元与进程测试、registry模板矩阵、Clippy、Rust1.89检查；原生ABI回显和统一入口验证。只验证默认无硬件模板；不改框架、不重复全工作区或硬件验证，不宣称完成Windows原生测试。
4. 记录每项证据及边界，执行python3 tools/check_versions.py --base HEAD和暂存检查，提交修复；使用--component分别执行--tag和--require-tag核对Pack/CLI新不可移动标签；根版本门禁照常检查，根未升版，其标签继续指向原发行提交（默认--require-tag要求标签指向HEAD，只适用于根升版提交，不能据此移动既有根标签）。最终报告版本、提交、标签和实际结果。

## 实施状态与证据

- 官方核实0.14.0、0.26.1、0.33.0均已发布且未撤回，0.33.0精确Rust API可访问。0.29.0尚未发行，不能用于独立夹具；ABI9代表改用实际发行的0.27.0，并由解析结果及头文件确认ABI9。此项是验收样本纠正，不改变设计或支持范围。
- 已实现按真实ABI选择文档，未知ABI失败时保留明确待完成说明；初步24项Pack单元测试通过，正在补真实registry矩阵与调用验证。


### 完成与验收证据（2026-11-12）

需求→实现→验收逐项核对：

| 需求/设计 | 实现 | 验收证据 |
| --- | --- | --- |
| 默认最新版new/init完成 | template::create沿用真实Cargo解析，按唯一节点ABI完成说明后报告成功 | registry默认用例的new/init均解析已发布0.33.0/v10，编译并完成原生调用，已有用户文件保留，重复init拒绝覆盖 |
| 保留旧版并支持当前ABI | template/abi.rs显式支持7–10；通用说明及两个ABI片段 | 0.14.0/v7、0.26.1/v8、0.27.0/v9、0.33.0/v10分别生成准确说明、--check不改锁，编译原生动态库；Python宿主先核对实际ABI，再执行GetFunctions/GetInitConfig/InitDevice/Echo/CloseAllDevices，全部成功 |
| 不猜未知ABI | ABI选择器拒绝0/6/11/u32::MAX；未完成说明不写具体签名 | 单元测试全部拒绝未知编号，逐版本核对文档无待解析标记、v10无三参数声明 |
| 真实依赖与文件保护 | 沿用精确registry依赖、Cargo生成锁及既有创建预检 | 两个原0.14.0集成用例通过，覆盖缺少cdylib/范围依赖/缺失说明、保留其他文件、拒绝覆盖、不构建且不改锁 |
| 统一入口可用 | CLI/套件最低Pack更新0.13.6 | 实际target/debug/cargo-miresidian new（无版本）与cargo-mires init（显式0.33.0）均成功，生成v10说明并通过项目检查 |
| 版本与交付指导一致 | Pack0.13.6、CLI/套件0.3.7，Cargo.lock仅这两个包版本变化，README/模板/CHANGELOG同步 | --base HEAD版本门禁通过；其他锁不含变更工具，无需改动，未升级任何独立Node/框架组件 |

执行结果：
- cargo test -p miresidian_pack -p miresidian_cli：24项Pack单元、两个CLI入口各3项单元、4项进程测试，共34项通过；4项registry用例按既有规则默认ignored。
- cargo test -p miresidian_pack --test template --locked -- --ignored --nocapture：联网显式执行4项全部通过（41.23秒），没有跳过latest验收。新矩阵共6个原生动态库调用项目（4个ABI代表版本、默认new和默认init）。
- cargo clippy -p miresidian_pack -p miresidian_cli --all-targets --locked -- -D warnings：通过。
- cargo +1.89.0 check -p miresidian_pack -p miresidian_cli --all-targets --locked：通过。
- RUSTDOCFLAGS='-D warnings' cargo doc -p miresidian_pack -p miresidian_cli --no-deps --locked：通过。
- cargo fmt -p miresidian_pack -p miresidian_cli -- --check、git diff --check：通过。
- cargo package -p miresidian_pack --list --allow-dirty：确认包含ABI选择模块、两类契约片段、通用模板及原生测试脚本；仅核对归档文件清单，不发布。
- python3 tools/check_versions.py --base HEAD：通过。提交前暂存门禁和提交后组件标签核验继续按本方案执行。

代码、文档和验收与方案一致。唯一样本调整是未发布0.29.0改为已发布0.27.0，已在实施前记录；无其他设计偏离。没有改变框架、宏、C ABI、租约协议、Node技能或既有节点；没有推送、发布或全局安装。本次是Linux宿主原生验证，未执行Windows原生加载及硬件测试；不以此声称完成这两类验证，Windows套件CI将复用新增跨平台registry回归。

不可移动标签：miresidian_pack-v0.13.6、miresidian_cli-v0.3.7；根miresidian-v0.33.0继续保持原发行提交5fb35784e14aca005868b39f6ba8858ba77de8b3。提交标识和标签核验结果以最终交付回复为准，避免在提交中写入自身哈希。
