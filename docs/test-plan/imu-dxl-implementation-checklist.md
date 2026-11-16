# IMU-DXL 首批适配代码验证交接

日期：2026-11-15；更新：2026-11-16。实现尚未提交、未部署。Linux 目标代码检查和指定范围 clippy 已通过，但 Rust 测试尚未实际执行，也未链接板端产物。不得拿设备现有版本号代替这次工作区代码的验证。详细范围见 [主机构建检查记录](imu-host-build-validation-20261116.md)。

## 2026-11-16 硬件信息补充

用户已确认当前型号 `qingmu-a1`、飞特协议及固定竖装方向；未来型号 `qingmu-b1` 的固定横装方向也已确认，但协议/数据格式待核实。型号、轴向及验收基准统一以 [适配方案补充](../design/imu-dxl-adaptation-plan.md#2026-11-16-补充用户确认的型号与安装约束) 为准，不在本页重复定义。下文“确认协议/安装方向”应理解为测试时核对实际设备与配置，而非仍等待用户提供方向。A1 型号自动解析安装旋转及冲突校验已写入代码，B1 仅有预设且禁止启用；新增测试代码已通过 Linux 目标编译检查，但尚未执行。后续 robotd/Agent/Studio 管理与五姿态标定已落入代码；其当前实现及未验证边界见 [统一交付记录](../design/imu-unified-delivery-goal.md)。

## 本轮管理闭环补充

已补 robotctl/Agent/Studio 管理闭环，旧文中“尚未包含 Studio”是首批交接范围。当前统一测试改用 [2026-11-16 管理闭环验收方案](imu-unified-acceptance-20261116.md)，其中明确可用功能、未完成内容、Linux 门禁和现场安全条件。下文保留首批硬件核对步骤，不作为最新界面进度。

## 已落地与限制

- 专用 IMU-DXL 解码器与飞特/Dynamixel 后端接线，复用 robotd 串口控制器。
- 来源 `imu-dxl` 和显式机体安装旋转；不创建 IMU IPC socket，不借用旧 SFLP 比例/安装角。
- 飞特按采样计数判冻结；Dynamixel 当前读取块无采样计数，只证明应答新鲜度，无法保证发现固件融合冻结。
- 已新增 8 项共用事务边界故障测试和 2 项读取失败状态回归测试；原始字节层的碎包/CRC/错误 ID 夹具仍待完成。未包含 Studio 型号选择自动应用、Agent 总线 IMU 状态桥接、型号库产物独立发布或硬件时序优化。
- 保留既有 IPC/embedded 路径，当前默认仍为 IPC，不自动识别/切换来源。

## 1. 主机优先验证

在具备仓库要求的 Linux/Rust 环境执行；依赖按 CONTRIBUTING.md 配置。推荐运行 `bash scripts/validate-imu.sh`，自动收集源码指纹、日志和结果；脚本不启动生产服务或访问真实设备；集成测试会启动 `--fake` 子进程。下列为手动检查命令。

```bash
cd zelux-robotd
cargo fmt --all --check
cargo test -p duck-control imu_dxl
cargo test -p duck-control
cargo test -p robotd-params imu_model
cargo test -p robotd-params
cargo test -p robotd
cargo clippy -p duck-control -p robotd-params -p robotd --all-targets -- -D warnings
```

首次运行如需格式修正，执行 cargo fmt 后审查差异；编译/语义/回归失败必须修复并重跑，不能跳过测试或用 fake 代替真实来源。当前已有格式检查、指定 Linux 目标编译/静态检查和 diff 检查；不等于上述 cargo test 已通过。完整通过范围与 Windows 平台阻塞见主机构建检查记录。

Agent D1 本机已先复现原有 2 条失败，再修复并执行：

```bash
cd zelux-device-agent/software
# 使用已安装独立 Drivers 的环境；否则无法覆盖原 catalog 集成缺陷。
python -m unittest zeluxd.tests.test_hal_selection zeluxd.tests.test_hardware_config -v
python -m zeluxd.tests.test_protocol_drivers
```

本机结果分别为 7 passed、15 passed；新增元数据错误和版本回归测试均包含在内。Python 语法检查通过。目标设备仍需更新 Agent 后重跑，历史结果保持原样。

## 2. 现场配置核对（不自动修改生产设备）

- 确认舵机使用的总线协议：飞特用 `feetech-sts`，Dynamixel 用 `dynamixel2`。不能因 IMU 支持两种协议就选择与舵机不同的协议。
- A1 配置包含 `[bus] protocol = "feetech-sts"`、`[bus.imu] source = "imu-dxl"`、`model = "qingmu-a1"` 和正数 `max_age_ms`（当前默认 100）；不必填写 `mount_wxyz`。无型号的历史自定义路径才必须显式填写安装四元数。
- `mount_wxyz` 表示传感器坐标到机体坐标的固定旋转，必须按实际板卡安装确定。仅在轴完全一致时才用 `[1, 0, 0, 0]`；未确认不得猜一个值启用运动。
- 无型号且缺安装旋转、全零/非有限旋转、max_age_ms=0、A1 搭配非飞特协议、A1 冲突旋转、已知 B1 未验证驱动或型号来源冲突时，应在加载参数时拒绝；开总线前另有校验。无型号合法显式旋转继续兼容。
- 同一 `/dev/ttyS2` 只能有一个所有者。测试前释放 Studio 总线接管和其他调试工具；不得另外运行 SDK/Agent 读取者与 robotd 抢串口。

## 3. 先只读，再动作

测试操作者应选择仓库既有且确认不会使能扭矩/自动 HOME 的诊断方式；不能假定直接启动 robotd 天然只读。没有确认安全模式、机械支撑和现场断电看护时，不启动真实控制。

需要补齐：

1. 构建提交/工作区补丁与二进制哈希；目标架构与配置路径；测试前服务和总线占用状态。
2. 当前协议下 ID 200 核心数据、就绪状态、计数（飞特）、接收频率；不发送舵机控制/EEPROM 写入到 ID 200。
3. 静止时重力方向正确，绕机体三个轴缓慢旋转时角速度正负、轴对应、rad/s 量级正确；XYZ 四元数为零且状态有效时可以 ready。
4. 未就绪、计数冻结（飞特）、坏帧、超时和 IMU 断线使来源 not-ready；恢复新样本后可恢复。不用修改主机系统时钟模拟单调时钟超时。
5. 关节和 IMU 读事务不会相互消费应答；测量成功/失败条件下的控制周期、抖动及超时预算。Dynamixel 每次额外三次读取的开销必须单独记录。
6. 完成独立 IPC、旧板 embedded 和总线接管回归；没有旧型号硬件则记录阻塞，不写通过。
7. 只有以上通过并具备运动安全条件，才做受控动作及故障状态的实际安全策略验证。ready=false 不能单独证明机器人动作已安全停止。

记录格式：用例、源码/产物、配置、实际结果、日志、通过/失败/阻塞。测试不修改现有历史执行证据，另附本轮时间和新结果。
