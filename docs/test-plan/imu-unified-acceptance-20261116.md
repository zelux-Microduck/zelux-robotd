# IMU 管理闭环统一测试方案

日期：2026-11-16。软件候选阶段交接：未部署、未做本轮硬件验收。机制定义以 [IMU 适配方案](../design/imu-dxl-adaptation-plan.md) 为准，阶段总结见 [交付目标](../design/imu-unified-delivery-goal.md)。另建本轮测试证据文件，不覆盖历史结果。

## 1. 本轮交付与边界

涉及 robotd、Device Agent、Studio、Protocol 四个独立仓库。共享 TTL 总线仍仅由 robotd 读取；Agent 只调用工具和 IPC，型号目录来自 robotd 参数层。

- 配置：支持 A1 固定安装配置；B1 展示但禁止启用；不自动改变舵机协议、不自动重启。
- 状态：保存配置、进程启动时加载配置、实际传感器 ready 分别显示。配置一致不等于硬件健康。
- 应用：明确安全确认后重启；新进程实例及配置 revision 均匹配才报告已加载，不把 systemctl 成功当硬件验收。
- 标定：总线 IMU 使用独立的 robotd 重力状态流多姿态对齐流程，Studio 支持采集、解算、revision 保护保存及之后独立重启；旧 Agent I2C 向导仍被禁用，不能拿另一颗传感器的数据代表总线 IMU。当前功能代码已落盘，但本轮按用户要求未运行测试、构建或设备验证。
- 型号由操作者确认，不是自动识别；不能仅从 ID 200 推断 A1/B1。

当前改动未提交、未发布。Device Agent、Studio、Protocol、robotd 的工作版本已分别更新到 1.5.0、1.3.0、1.4.0、0.15.0；robotd 尚未生成并部署新的构建产物。版本号不证明本轮代码已验证或运行。

## 2. 本机实际自动验证

| 范围 | 实际结果 | 不代表 |
|---|---|---|
| Agent 桥接、进程清理、HAL、引脚与协议驱动（前一阶段记录） | 42 项通过，其中本轮新增 20 项 | 不覆盖本次新增的 IMU 标定复合逻辑；设备权限/systemd 验收 |
| Agent 固件管理 | 7 项回归通过 | 已刷写固件 |
| Studio 状态规则 | 9 项断言通过 | 浏览器视觉验收 |
| Studio 实际控件、确认与命令载荷 | 轻量 DOM 集成测试通过 | 真实设备/WebSocket 联调 |
| Protocol | Schema 定义与三份匹配示例严格验证通过 | 设备实测 |
| Rust 生产代码 | ARM64 Linux 参数库、robotctl、robotd 代码检查和严格 clippy 通过 | 链接/生成可部署程序 |
| Rust 含测试代码 | ARM64 Linux 参数库、控制库、robotctl all-targets 检查及严格 clippy 通过 | Rust 测试实际执行 |
| 构建打包（前一阶段记录） | Studio 1.2.0、Protocol 1.3.0 本地打包成功；Agent 1.4.0 版本一致性检查通过 | 不代表本轮 Studio 1.3.0、Protocol 1.4.0、Agent 1.5.0 或 robotd 0.15.0 已构建/发布 |
| 脚本与差异 | Linux 验证脚本语法、各仓库 diff 检查通过；51 项源码指纹复核一致 | 远端 CI 已执行 |

上述是前一阶段已经记录的验证结果，**本次多姿态标定功能收口后未重新运行任何自动测试、构建或浏览器检查**，也未部署或连接设备；不能把前列历史通过项目等同于本次新改动通过。

当前 Windows 环境不能原生运行现有 Unix 时钟/IPC 测试。robotd 全目标交叉检查的测试依赖需要 aarch64-linux-gnu-gcc，当前缺少；未绕过失败或生成占位产物。下一关必须先在 Linux 执行。

## 3. 第一关：Linux 无硬件验证

不连接真实串口、不启动生产服务、不使用设备默认 socket。按 CONTRIBUTING.md 安装依赖，在 robotd 仓库执行：

~~~bash
bash scripts/validate-imu.sh
~~~

脚本执行参数/控制/robotd/robotctl 测试、严格检查与本机链接；部分集成测试启动私有 socket 的 --fake 子进程。收集 target/imu-validation/run-*。核对新增用例：A1 配置、B1/错误协议拒绝、文件保持、revision 冲突、注释/其他字段保留、CLI JSON 往返。失败先修复，不进入动作测试。

Agent 仓库安装独立 Drivers 包；开发时也可将 software 和相邻 Drivers/src 加入 PYTHONPATH：

~~~bash
python -m unittest zeluxd.tests.test_robotd_imu_management zeluxd.tests.test_robotd_imu_process zeluxd.tests.test_hal_selection zeluxd.tests.test_hardware_config zeluxd.tests.test_protocol_drivers -v
python -m zeluxd.tests.test_firmware
~~~

Studio 仓库：

~~~bash
node --experimental-default-type=module tools/test-imu-management.mjs
node --experimental-default-type=module tools/test-imu-management-dom.mjs
python tools/package.py
~~~

Protocol 仓库：

~~~bash
python -m pip install -r requirements-dev.txt
python tools/validate.py --schemas
python tools/package.py
~~~

## 4. 第二关：构建与部署准备

第一关通过后再使用仓库现有 Linux/ARM64 构建发布路径，至少包含同次工作区的 robotd 与 robotctl。本方案不自动发布/部署，也不跳过现有更新健康门禁。

| 组件 | 要求 |
|---|---|
| robotd / robotctl | 同次构建，IPC API 32；支持 imu-config、imu-status |
| Protocol | 工作区版本 1.4.0；1.x 协商保持向后兼容 |
| Agent | 工作区版本 1.5.0，含管理桥接、命令、状态和多姿态标定 |
| Studio | 型号卡片、保存/重启分离、未知状态处理 |
| Drivers | 原 Agent HAL 包；并非供 robotd 动态加载的 Rust 包 |

核对 Agent 环境变量默认值：

- ZELUX_ROBOTCTL=/opt/robot/daemon/current/bin/robotctl
- ZELUX_ROBOTD_CONFIG=/etc/robot/robotd.toml
- ZELUX_ROBOTD_SOCKET=/run/robotd.sock
- ZELUX_MODE=radxa

Agent 需具备 IPC、指定配置写入、重启 robotd.service 的最小权限，不要为绕过测试开放任意系统命令权限。Agent 路径必须与运行进程回报一致，否则拒绝保存。重启使用固定服务名，不接受前端 shell 命令。

备份原配置、安装版本/哈希、服务及总线占用状态。不能为读取状态就随意启动真实 robotd，更不能假设直接启动天然不会动作。

## 5. 第三关：只读观察与界面验收

这些管理命令本身不打开串口、不启动服务：

~~~bash
/opt/robot/daemon/current/bin/robotctl imu-config --file /etc/robot/robotd.toml
/opt/robot/daemon/current/bin/robotctl --robot-socket /run/robotd.sock imu-status
~~~

| 编号 | 场景 | 预期 |
|---|---|---|
| U01 | 打开 IMU 页 | 配置/运行型号分开；显示驱动/版本、固定安装配置、健康 |
| U02 | A1 固定安装方向 | 与机制文档一致，不提供任意旋转编辑 |
| U03 | B1 | 明确未验证，界面不能启用，直接调用后端也拒绝 |
| U04 | 旧工具/旧 robotd | 缺少能力/未知；不伪报生效，不用 I2C 读数冒充总线 IMU |
| U05 | 服务离线、工具/socket 权限不足 | 错误可见、ready=false、禁止写入；恢复后可刷新 |
| U06 | 状态超过 10 秒未更新 | 未知，不保留旧 ready 或已生效判断 |
| U07 | --fake / --sim 实例 | 标明模拟，不能作为硬件生效证据，不能写入/重启 |
| U08 | 配置路径不一致 | 拒绝，不修改错误文件 |
| U09 | A1 或待应用 A1 的标定 | 旧 I2C 采集/solve/commit/bias/sample 仍拒绝；独立五姿态流程仅在 A1 当前运行配置已生效且 ready 时开放；采集期间 revision 变化必须中止；标定修正待重启，不能冒充已应用 |
| U10 | 已确认仍用 IPC 的环境 | 保留原 I2C 向导；结果不能作为 A1 标定证据 |
| U11 | Studio 标定结果 | 显示本次增量修正与保存的完整修正；机器人坐标映射单独显示 robotd 回报的运行组合旋转；状态过期显示未知，不保留陈旧“已标定”结论 |

离线/模拟/路径错误优先用隔离实例注入，不为界面测试中断正在运动的生产进程。无旧板/IPC 硬件时记阻塞，不能填通过。

## 6. 第四关：保存与受控重启

写入/重启前必须停止运动、提供机械支撑并安排现场断电看护。robot.safeToRestart 仅是软件忙闲检查，不是物理安全证明。检查到重启之间没有运动锁租约，整个操作期间其他客户端不得下发动作。

| 编号 | 场景 | 预期 |
|---|---|---|
| C01 | 取消保存 | 文件/服务不变 |
| C02 | 飞特总线保存 A1 | source=imu-dxl、model=qingmu-a1；移除旧安装覆盖，保留超时、舵机协议、其他字段及注释 |
| C03 | 保存后重启前 | 原实例不变，显示保存/运行差异与待重启 |
| C04 | 并发修改后提交旧 revision | 冲突拒绝，不覆盖另一个客户端的新值 |
| C05 | 非飞特协议、B1、未知型号、非法超时 | 拒绝，不自动改共享协议，不破坏原文件 |
| C06 | 取消重启或确认字段不全 | 不调用 systemctl |
| C07 | robotd 回报不允许重启 | 拒绝，无服务操作 |
| C08 | 正常重启 | 新 instance_id，运行 revision 与保存配置一致；ready 单独核对 |
| C09 | 重启命令失败/超时，未读到新实例 | 失败/待确认，不伪报成功，不自动循环重启 |
| C10 | 新进程加载旧配置 | 不判定应用成功，保留差异 |
| C11 | 配置一致但传感器未就绪 | 同时显示配置一致与未就绪，不宣称整机正常 |

若需回退，由操作者在安全条件下恢复配套配置与产物；旧程序不认识 A1 配置时不能只退程序。操作失败应先核对真实状态，不盲目重试。

## 7. 第五关：传感器与共享总线

按 [硬件验证交接](imu-dxl-implementation-checklist.md) 验证 A1 单位、三轴正负、姿态/重力、计数冻结、未就绪、失联恢复及与舵机共存的时序。不得向 ID 200 发送舵机控制/EEPROM 写入。

原始字节层拆包、粘包、噪声、错误 ID、CRC 注入的自动化夹具仍未完成，已有事务层测试不覆盖它。只在隔离串口夹具测试，无夹具记阻塞，不对运行中的总线随意注入或带电拔插。

只有前述通过且具备现场安全条件，才开展受控动作和真实故障安全策略测试。ready=false 不等于动作已安全停止。读取频率、周期抖动、超时预算和舵机/IMU 应答隔离需实测，不因页面在线而跳过。

对齐标定的后续专项步骤：确保机器人有机械支撑、停止动作且现场有人看护；在 A1 已加载、ready 的状态下依序采集机体立正、前倾、后倾、左倾、右倾，每次保持静止且倾角 12°–65°；完成后检查方向残差和完整 WXYZ 修正，保存后应显示待重启而非已应用；安全重启后核对新进程 revision 和实际组合旋转。若已有标定，再采一轮不同的小偏差姿态，确认重新解算后是增量复合而不是覆盖/叠加重复；改变型号或配置 revision 后应要求重新开始。首次真机应用应先用手动静态姿态检查机体前/左/上方向和投影重力，再决定是否进入任何动作验收。

## 8. 报告与暂停条件

每例记录：编号、四仓库源码指纹、二进制哈希、配置路径/内容、实际结果、日志/截图、通过/失败/阻塞及定位。编译通过不写为测试执行通过，旧设备证据不复用成本轮结果。

通过标准：Linux 门禁、配套产物、U/C 用例和 A1 硬件验收全部完成。安全、轴向、总线冲突、错误生效判定任一失败立即停动作测试。

本轮标定界面与配置链路已实现到代码候选阶段；按用户要求暂不测试，之后由用户统一测试或遇到具体问题再修复。仍有独立工作：B1 协议适配、原始字节注入夹具、Rust 适配包独立版本/发布、ARM64 发布/签名/回滚验收。此验收文档保留为后续验证方案，不表示本轮已验证通过。

本轮源码核对入口：[源码指纹清单](imu-management-source-manifest-20261116.json)。包含四个开发仓库及参与回归的 Drivers 仓库 HEAD 和变更源码哈希，可能包含此前未提交改动；不包含构建产物，也不代表所有变更均由本轮引入。文档不纳入源码指纹以免互相引用产生自校验循环。
