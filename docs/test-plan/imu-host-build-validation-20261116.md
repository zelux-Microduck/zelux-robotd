# IMU 主机构建检查记录（2026-11-16）

## 范围与结论

本轮继续完成构建前检查和故障回归代码，不部署、不打开设备串口、不启动 robotd、不使能电机。主机为 Windows，Rust 工具链现已可用：rustc 1.100.0（0ef1bc386，2026-11-09）、cargo 1.100.0（3acc79c0c，2026-11-05）。仓库基准提交为 `22fcca3b0d3dfe390707e9668200f695197c2059`，验证对象是带未提交修改和新增文件的工作区，不是该提交本身。

**已完成格式检查、Linux x86_64/ARM64 目标代码检查和指定范围的严格静态检查；尚未完成 Linux 上的 Rust 测试执行、二进制链接或硬件验证。不得将 cargo check 或 clippy 写成 cargo test 通过。**

## 本轮代码补充

- 新增 `duck-control/src/imu_dxl/transaction.rs`，由真实飞特/Dynamixel 后端共用，只接收既有控制器的读取回调，不自行打开串口或提供写操作。
- 8 项事务测试覆盖固定 ID/地址/长度、单一读取器上的调用顺序、缺失/多余/短/超长返回、每一步读取失败的短路、前后任意一次 not-ready、失败后重复计数不能恢复，以及串口耗时计入样本年龄。
- 这些夹具位于已解析响应的事务边界；报文碎片、CRC、错误设备 ID 的原始字节级验证仍属于后续串口模拟/库集成测试，不能声称本轮已覆盖。
- 修复 SplitImuIo 在总线读取或来源采样失败时可能保留旧 ready 的问题：先将 IMU 标记不可用并增加失败计数，再向上传递原始错误；不覆盖上次诊断数据，不伪造有效姿态。新增 2 项失败及恢复回归测试。
- 修复静态检查发现的默认实现可派生和枚举尺寸告警。IPC 来源仅在初始化时分配一次 Box，不在控制循环中逐帧分配。

## 实际执行结果

以下命令在仓库根目录执行；本机使用工具链绝对路径调用 cargo，对应 stable。表中的通过仅适用于明确列出的范围。

| 检查 | 结果 | 边界 |
|---|---|---|
| `cargo fmt --all --check` | 通过 | 已先修正格式；包含新增 Rust 文件的语法解析 |
| `cargo test --locked -p robotd-params`（Windows 原生） | 失败/平台阻塞 | 既有 duck-ipc-proto 时钟代码依赖 libc::clockid_t、clock_gettime、CLOCK_MONOTONIC/REALTIME；未为测试改动 Unix 时钟语义，未执行到测试用例 |
| `cargo check --locked -p robotd-params -p duck-control --all-targets --target x86_64-unknown-linux-gnu` | 通过 | 包含两包测试代码的类型检查，不链接或执行测试 |
| `cargo clippy --locked -p robotd-params -p duck-control --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` | 修复后通过 | 首轮发现上述两处静态检查问题，未屏蔽告警 |
| `cargo check --locked -p robotd-params -p duck-control -p robotd --lib --bins --target aarch64-unknown-linux-gnu` | 通过 | robotd 及参数/控制库的 ARM64 Linux 生产代码；未链接产物 |
| `cargo clippy --locked -p robotd-params -p duck-control -p robotd --lib --bins --target aarch64-unknown-linux-gnu -- -D warnings` | 通过 | 同上范围；不包含 robotd 的完整集成测试目标 |
| `cargo check --locked -p robotd-params -p duck-control --all-targets --target aarch64-unknown-linux-gnu` | 通过 | 参数/控制库含测试代码的 ARM64 目标检查 |
| `bash -n scripts/validate-imu.sh` | 通过 | 脚本语法；未在本机运行 Linux 完整门禁 |
| 在 Windows Git Bash 执行验证脚本 | 按设计退出 2 | 拒绝将非 Linux 环境当成完整 Linux 测试；未启动任何设备程序 |
| Agent HAL/硬件配置/驱动管理回归 | 7 + 15 项通过 | 使用独立 Drivers 路径重跑，属于 Python 组件检查，不替代 Rust 或硬件验收 |

## Linux 上的下一步（不是整机测试）

在具备 CONTRIBUTING.md 所述依赖的 Linux 开发机/CI 环境执行：

```bash
bash scripts/validate-imu.sh
```

脚本运行格式检查、参数库/控制库/robotd 测试、严格静态检查及主机二进制构建。所有测试仍是仓库的无硬件回归；robotd 集成测试会启动带 `--fake` 的测试子进程并使用私有 socket，不启动生产服务、不部署、不连接机器人。已有 CI 的 workspace 测试继续保留，并新增该脚本的 bash 语法/shellcheck 检查。

证据输出到 `target/imu-validation/run-*`：检查日志、最终退出码、基准提交、工作区状态、已跟踪修改补丁、含未跟踪源码的文件哈希清单，以及成功构建后的主机二进制哈希。失败不会标记 PASS；主机产物也不能冒充板端发布产物。必须在 Linux 上真正执行 Rust 测试后，再安排安全只读的设备采样和总线时序测试。

仍待完成：Linux 实际单测/集成测试执行和链接产物；原始串口帧故障集成；板端时序与受控硬件验收；Agent/Studio 型号配置和 health 生效回读；B1 协议确认。型号固定安装约束以 [适配设计](../design/imu-dxl-adaptation-plan.md) 为准。

## 本轮主要源码指纹

下列为记录时的文件字节 SHA-256，包含新增的未跟踪文件。它们不是发布二进制哈希；后续源码或换行变化后需重做检查并生成新证据。

| 文件 | SHA-256 |
|---|---|
| `duck-control/src/imu_dxl.rs` | `e9e152b70e1db8c358e740cecade00899c769ac2060b9a0c0c7e21f7020f11ef` |
| `duck-control/src/imu_dxl/transaction.rs` | `1fc9f5579a6a5467044638522fc5180f356295324d8b57b4d134e627d0b518bd` |
| `duck-control/src/imu_source.rs` | `40cb2b8afa2e4d1f7c69ddf954f1a1edaad8fc20ac794fd22a2e2920a8a72a21` |
| `duck-control/src/bus.rs` | `9e76b993b325a1c2caa6e4c863e559a95b3036c05943e7c575db0d6a98c856cc` |
| `duck-control/src/bus_feetech.rs` | `fbeb9c42aa0072ba98fc3d4e987527a7e6bff19e2244fa479ea45ef947a40a9e` |
| `robotd-params/src/imu_model.rs` | `7cf5b5a575cbe667a2e608db597ad44deae221d1197c1046508c4eba4e4690e4` |
| `robotd-params/src/lib.rs` | `12a8da705d8635a10eee8b881c77ea705f5ac202653aff342e36c3fb25f0c447` |
| `robotd/src/main.rs` | `d23f7048a0f4fb0710a3463efd1c89e9d0900fc40bfac6b457910c40ee815996` |
