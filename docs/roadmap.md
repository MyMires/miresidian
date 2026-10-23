# 后续优化备忘录

本文件记录已确认、但尚未进入当前发布版本的工程化事项。

## 国际化发布信息

- 提交说明优先使用中文。
- 公开发布前，为 README、CHANGELOG 和 Release Notes 提供中英文版本。
- 若未来建设文档站点，再根据浏览器语言自动选择中文或英文页面；Git 提交信息与 Git 托管平台页面不支持自动切换。

## 节点流程测试工具

- 当前 `miresidian_testflow_windows` 只负责将 `.testflow` 生成 Windows BAT 入口；流程执行仍由节点项目持有的 PowerShell 调用器完成。
- 当 `.testflow` 的跨平台语法、断言和报告格式稳定后，再抽取 `miresidian_testflow` 核心 crate；在此之前不重复实现或猜测流程语法。
- 工具应通过 C ABI 的 `GetFunctions` 与 `GetInitConfig` 自适应节点，而不包含串口业务规则。
- 保留节点项目对流程文件、测试数据和硬件环境的所有权。
