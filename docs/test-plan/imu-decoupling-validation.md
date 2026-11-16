# robotd 与 IMU 解耦测试执行单

> 2026-11-15 设备信息修订：用户确认当前为共享舵机总线的 IMU-DXL。下文保留已有测试事实；无 ID 200 的要求只适用于独立 IPC 场景，不能作为该型号的通用验收条件。共享总线适配、差异和新增测试计划见 [IMU-DXL 修订方案](../design/imu-dxl-adaptation-plan.md)，适配尚未实现。

## 1. 测试目的

验证当前实现满足以下目标：

- `robotd` 默认通过本机 Unix datagram socket 接收独立 IMU 驱动数据；
- `robotd` 在独立 IMU 模式下不再读取 Dynamixel ID 200；
- 旧设备仍可通过 `source = "embedded"` 兼容运行；
- IMU 驱动重启、数据过期、非法数据和重复发布者不会导致错误数据进入控制环；
- `zeluxd` 能加载所选 IMU 驱动并发布符合 `zelux.robotd-imu/v1` 契约的数据；
- socket 权限、服务启动顺序和单实例锁满足部署要求。

本文件是执行单，不要求测试人员修改代码。每条用例记录“通过/失败/不适用”、设备型号、软件版本、日志片段和时间。

## 2. 测试范围与前置条件

### 2.1 测试对象

- `zelux-robotd`：`robotd`、`robotd init`、参数文件；
- `zelux-device-agent`：`zeluxd`、IMU 驱动选择和 `ImuPublisher`；
- `zelux-protocol`：`robotd-imu/v1` schema；
- 目标板上的 systemd 服务和 `/run/zelux/robotd-imu.sock`。

### 2.2 两套配置

独立模式（主路径）：

```toml
[bus.imu]
source = "ipc"
socket = "/run/zelux/robotd-imu.sock"
max_age_ms = 100
```

兼容模式（旧硬件）：

```toml
[bus.imu]
source = "embedded"
```

测试开始前确认舵机处于安全姿态，旁边有人可以立即断电；涉及运动的测试只允许在空旷区域进行。

### 2.3 建议采集的信息

```bash
systemctl status robotd zeluxd --no-pager
ls -l /run/zelux/robotd-imu.sock
id zelux
journalctl -u robotd -u zeluxd -n 200 --no-pager
```

## 3. 主机自动化测试

在安装了 Python 依赖的环境执行：

```bash
cd zelux-device-agent
python -m pytest software/zeluxd/tests/test_imu_publish.py -q
python -m pytest software/zeluxd/tests/test_hal_selection.py -q
python -m compileall software/zeluxd

cd ../zelux-device-drivers
python -m pytest tests/test_imu_contract.py tests/test_catalog.py -q

cd ../zelux-protocol
# 使用仓库现有 schema 校验工具，确认 examples/robotd-imu.json 可通过
```

验收：所有测试通过；没有未处理异常；schema 校验通过；测试输出和 Python 版本写入结果单。

若目标环境具备 Rust 工具链，再执行：

```bash
cd zelux-robotd
cargo fmt --check
cargo test -p duck-control
cargo test -p robotd-params
cargo test -p robotd
```

## 4. 设备测试用例

### T01：独立模式正常启动

1. 设置 `source = "ipc"`，重启 `zeluxd` 和 `robotd`。
2. 确认 socket 存在，属主/权限类似 `root:robot 0660`。
3. 观察 `robotd` 健康状态和 IMU ready 状态。
4. 让机器人保持静止，再执行一次受控的小幅动作。

预期：两个服务均为 active；IMU 数据持续更新；robotd 不报告 ID 200 缺失；动作无异常抖动；IMU stale 状态为 false。

### T02：确认 robotd 不依赖 ID 200

在独立模式下断开旧 ID 200 IMU（或使用没有 ID 200 的总线），保留舵机和独立 IMU 驱动。

预期：robotd 仍能启动并进入 ready；舵机可读写；日志明确使用 IPC IMU，而不是尝试读取 ID 200。

### T03：旧 embedded 兼容模式

1. 将配置改为 `source = "embedded"`。
2. 确认旧 ID 200 IMU 在线，重启 robotd。
3. 检查启动、健康状态和一项小幅动作。

预期：旧硬件正常工作；robotd 使用 embedded IMU；不要求 `/run/zelux/robotd-imu.sock` 有发布者。

### T04：IMU 驱动选择与型号显示

在 Studio/设备管理界面选择一个已安装的 IMU 型号，记录型号、驱动 ID、版本和兼容性；随后切换到另一已安装型号。

预期：选择结果持久化；`zeluxd` 启动后加载新驱动并开始发布；不兼容驱动不可选或明确显示原因；切换不会修改舵机驱动配置。

### T05：驱动进程停止后的 stale

```bash
systemctl stop zeluxd
```

等待超过 `max_age_ms`（建议至少 1 秒），观察 robotd 健康状态；不要在 stale 状态下继续让机器人运动。

预期：IMU 被标记为 stale/not-ready；robotd 不使用最后一帧无限运行；不会自动回退到 embedded；日志含清晰原因。

### T06：驱动重启与 publisher 重置

停止并重新启动 `zeluxd`，确认新进程生成新的 `publisher_id`，sequence 可以从 0 重新开始。

预期：robotd 接受新发布者的首帧和后续递增序列；不会因旧 publisher 的 sequence 较大而永久拒绝新进程数据。

### T07：非法/过期数据防护

使用测试工具或现有单测向 socket 发送以下数据：错误 schema、缺少字段、NaN/Inf、超大陀螺仪、错误四元数范数、错误 publisher、sequence 倒退。

预期：非法帧被拒绝且不进入控制环；robotd 保持上一有效状态或转为 not-ready；日志可区分校验失败原因；进程不崩溃。

### T08：重复实例和 socket 安全

1. 在 robotd 已运行时再次启动 robotd。
2. 将 socket 路径替换为普通文件后尝试启动。
3. 清理残留 socket 后重新启动。

预期：第二个实例被拒绝；不会覆盖已有 identity 或删除普通文件；可安全清理 stale socket 后恢复启动。

### T09：无权限用户访问

确认 `zelux` 用户属于 `robot` 组，再以该用户运行发布端；再用不属于该组的普通用户尝试访问 socket。

预期：合法发布端可以发送；无权限用户被系统拒绝；权限不会通过 `0777` 等方式放宽。

### T10：robotd init 与总线互斥

分别在 robotd 运行时、robotd 停止时执行 `robotd init`。

预期：robotd 运行时 init 被锁拒绝；robotd 停止时 init 可独占总线完成动作；不会出现两个进程同时控制舵机。

## 5. 最新结果（2026-11-15 22:45–22:51，GMT+8）

本轮通过 SSH 在 192.168.2.117 执行。第 8 节保留早期历史记录，其中“robotd 未安装”“无 UART2”“无 robot 组”已不再代表当前状态；本轮证据和限制见第 9 节。

设备：Radxa Zero 3W / Armbian 26.8.1 trixie / Linux 6.1.115-vendor-rk35xx / Python 3.13.5。
部署版本：Agent 1.4.0、Drivers 1.2.0、Protocol 1.2.0、robotd 0.14.1。

| 用例 | 本轮结果 | 实际覆盖与证据 | 未完成项/原因 |
|---|---|---|---|
| T01 | 阻塞，未通过完整验收 | 已确认 IPC 配置、版本、UART2 和 Agent 状态；日志报告 IMU 未探测到 | 未启动真实 robotd、未重启 Agent、未做动作；没有有效 IMU，现场安全姿态/断电看护未确认 |
| T02 | 阻塞 | 舵机总线已恢复；此前 Studio 日志曾报告 15/15 在线 | 未对真实 robotd 进行总线事务观测，未证明没有 ID 200 访问或 ready |
| T03 | 阻塞 | 未修改 embedded 配置 | 未提供并确认在线的旧 ID 200 IMU，未测试兼容硬件和动作 |
| T04 | 阻塞；相关回归不通过 | HAL 选择 unittest：1 通过、2 失败；见 9.2 | 遵循第 6 节，T01/T02 未通过前不做 Studio 驱动切换；未验证型号切换/持久化 |
| T05 | 阻塞 | 当前真实 robotd 停止，实际 IMU socket 不存在 | 没有接收端有效帧基线，未停止生产 Agent；不能验证 stale 对控制环的作用 |
| T06 | 部分通过，接收侧阻塞 | 以 zelux 运行两次实际 ImuPublisher，合成 HAL + Python 接收器；各收到 15 帧，publisher_id 不同，sequence 从 0 递增 | 未重启生产 zeluxd，也未证明 robotd 接受新发布者 |
| T07 | 阻塞 | 未注入真实 IMU 接收端，未运行 Rust 测试 | 没有安全可用的真实接收端；--fake 绕过物理 IPC IMU，不能作为替代；各非法帧项均未验收 |
| T08 | 控制 socket 三项通过；IMU socket 未覆盖 | 真机已安装二进制 + 隔离 --fake 实例：重复实例退出 1；普通文件保持原文；残留控制 socket 可重建 | 未验证 IpcImuSource 的 datagram socket 安全；不能据此宣布完整 IMU 验收通过 |
| T09 | 部分通过，部署端阻塞 | zelux 属于 robot 组；隔离 root:robot 0660 datagram fixture：zelux 可发送、nobody PermissionError | fixture 是 Python 接收器，不是生产 robotd socket；实际 socket 权限未验收 |
| T10 | 锁拒绝分支通过；完整用例阻塞 | 隔离 fake daemon 持锁时 init 退出 1：another robotd owns this socket；另指定不存在的串口作为防线 | 未执行停机后 init 的上力/HOME 分支，未在真实总线上执行动作 |

**结论：本轮未达到第 7 节的主路径或兼容路径通过标准。** T01–T10 已逐项记录状态，但未执行的硬件/运动步骤仍为阻塞，不计为通过。隔离测试不是物理 IMU 或整机验收。生产服务保持测试前状态，未改协议或绕过校验。

## 6. 失败处理规则

- 任何涉及运动安全的失败，立即停止动作并保留完整日志，不继续后续运动用例。
- 若 T01/T02 失败，先不要测试 Studio 切换；优先确认配置、服务日志、socket 权限和硬件连接。
- 若只有 T03 失败，说明旧兼容路径有回归，但不应把配置改回 embedded 作为独立模式的长期方案。
- 若 T05 失败，属于高优先级安全问题：不能接受 stale IMU 继续驱动控制环。
- 测试人员只报告现象和日志，不在测试过程中修改协议或绕过校验。

## 7. 通过标准

主路径通过条件：T01、T02、T05、T06、T07、T08、T09 全部通过；T04 至少完成一个 IMU 型号的选择和重启验证。

兼容路径通过条件：T03 和 T10 通过。

在 Rust 工具链不可用的机器上，可以先完成设备与 Python 测试，但最终发布前必须在 Linux CI 或目标板补齐 Rust 编译、格式检查和单元测试。

---

## 8. 执行记录（2026-11-15）

### 8.1 环境基线

| 项 | 实测值 |
|---|---|
| 设备 | Radxa Zero 3W，aarch64 |
| 系统 | Armbian 13 (trixie)，Python 3.13.5 |
| 部署版本 | `/opt/zelux/current -> releases/1.0.0`，`/api/health` = `{"ok":true,"version":"1.4.0","mode":"radxa"}` |
| zeluxd | active，8000 端口 HTTP 200 |
| robotd | **未安装**（`command -v robotd` 无输出；无 `robotd*` systemd 单元；无 `/etc/robotd`） |
| Rust 工具链 | **无**（`cargo`/`rustc` 均不存在；`rustup` 未安装） |
| IMU 硬件 | **无**（zeluxd 日志：`没找到 IMU（i2c-3 上没有 0x6B/0x68/0x69/0x6A 应答，IIO 也没有）`） |
| 舵机总线 | **无**（仅 `/dev/ttyS1` 作为 console；无 `/dev/ttyS2`；无 `/dev/zelux-servo`） |
| IPC socket | **不存在**（`/run/zelux` 目录本身就不存在） |
| 硬件资源 | 磁盘 12G 可用；内存 963MiB（可用 760MiB）；有 gcc/make，**无** pkg-config / cmake / protoc / libdbus-1-dev / libasound2-dev / libudev-dev |

### 8.2 第 3 节：主机自动化测试结果

在第 3 节要求的 Python 环境下执行（板端 `/opt/zelux/venv` Python 3.13.5，pytest 9.2.0）：

| 命令 | 结果 |
|---|---|
| `pytest software/zeluxd/tests/test_imu_publish.py -q` | **通过**：2 passed |
| `pytest software/zeluxd/tests/test_hal_selection.py -q` | **不通过**：2 failed, 1 passed |
| `python -m compileall software/zeluxd` | **通过**：rc=0，无语法错误 |
| `pytest tests/test_imu_contract.py tests/test_catalog.py -q` | **通过**：5 passed |
| `python tools/validate.py`（zelux-protocol） | **通过**：`protocol 1.2.0: 6 files`，schemas + examples 全部 OK |
| Rust：`cargo fmt --check` / `cargo test -p duck-control` 等 | **未执行**：本机与设备均无 Rust 工具链 |

按第 3 节验收标准"所有测试通过"，**本次不满足**，失败项如下。

#### D1（P1，真实缺陷）驱动元数据查询失败会把已构造成功的驱动判为构造失败

失败用例：`test_selected_factory_controls_each_hal_part`、
`test_start_failure_falls_back_only_failed_device`。

实测输出：

```
AssertionError: 'zelux.tof.st-vl53l5cx' != 'zelux.tof.custom'
ERROR test.hal:radxa.py:106 tof 驱动 zelux.tof.custom 构造失败（'未知驱动：zelux.tof.custom'），回退默认驱动

AssertionError: '启动失败' not found in "构造失败：'未知驱动：zelux.tof.custom'"
```

根因（代码已复核）：`zeluxd/hal/radxa.py:92-106` 的 `_construct()` 里，
`self._factory(wanted, ...)` **已经构造成功**之后，第 99 行
`version = driver_descriptor(wanted).version` 去 catalog 查描述符；
`catalog.descriptor()` 在未登记时抛 `KeyError("未知驱动：…")`（`zelux_drivers/catalog.py:26-30`），
于是整个 `try` 落入 `except`，**已构造好的实例被丢弃并回退默认驱动**，
`driver_report` 里记为"构造失败"。

影响：任何"能创建但未登记描述符"的驱动都会被静默替换为默认驱动。
现场安装的第三方 `.zdp` 驱动、以及 T04 要验证的"选择一个已安装的 IMU 型号"，
都会命中这条路径——选了 A 却加载 B，且只在日志里留一行 ERROR。

建议修复：把描述符查询与实例构造解耦，查询失败仅令 `version = None` 并记录告警，
不要影响已经成功的构造；第二个用例是同一根因的连带失败（构造阶段就失败，
`start()` 的"启动失败回退"分支根本没被走到），修 D1 后需重跑确认。

### 8.3 T01–T10 逐条执行记录

每条都按执行单步骤实际尝试，记录"执行到了哪一步、为什么不能继续"。

**T01 独立模式正常启动 → 阻塞**
- 步骤 1（设 `source = "ipc"`，重启 zeluxd/robotd）无法执行：`/etc/robotd` 不存在，
  没有 `robotd.toml`，也就没有 `[bus.imu]` 段可改；`robotd` 不是可执行命令。
- 步骤 2（确认 socket `root:robot 0660`）：`/run/zelux` 目录都不存在，`robotd-imu.sock` 无从产生。
  zeluxd 的发布端在 `sendto` 遇 `FileNotFoundError` 时按"robotd 尚未选择 IPC 源"静默跳过
  （`imu_publish.py:84-86`），所以现场也不会报错，只能靠人工确认。
- 步骤 3/4（健康状态、动作）：robotd 健康状态不存在；无舵机总线，无法执行动作。
- 附：即使 robotd 就位，本机无 IMU 硬件，`packet()` 因 `snapshot["ok"]` 为假返回 `None`，
  不会发布任何帧，"IMU 数据持续更新"依然无法成立。

**T02 确认 robotd 不依赖 ID 200 → 阻塞**
- 需要"有舵机、无 ID 200"的总线。本机既无 Dynamixel 适配器也无 `/dev/ttyS2`，
  无法构造该硬件组合；robotd 不存在，也无法观察其日志是否尝试读取 ID 200。

**T03 旧 embedded 兼容模式 → 阻塞**
- 改 `source = "embedded"` 的前提是存在 robotd 配置与 robotd 进程，两者都没有。
- 且 embedded 路径必然读取 ID 200，而本机没有任何 Dynamixel 设备在线。

**T04 IMU 驱动选择与型号显示 → 阻塞（机制侧不通过）**
- 需要至少一个"已安装的 IMU 型号"。本机 i2c-3 上无任何 IMU 应答，无型号可供选择，
  更没有可观察的"加载新驱动并开始发布"。
- 该用例依赖的 HAL 驱动选择路径已在单测中被证伪（见 8.2 D1）：
  选择非 catalog 登记的驱动会被静默回退到默认驱动。**建议先修 D1，再用真机重测本条**。

**T05 驱动进程停止后的 stale → 阻塞**
- `systemctl stop zeluxd` 可执行，但"IMU 被标记 stale"是 robotd 侧状态，
  本机无 robotd，无健康接口可读，无法判定"不使用最后一帧无限运行"。
- 唯一可确认的是 zeluxd 停止后发布端确实消失（进程退出即无 publisher），
  但接收侧行为完全未覆盖。

**T06 驱动重启与 publisher 重置 → 阻塞**
- `ImuPublisher` 每次实例化生成新的 `uuid4().hex`（`imu_publish.py:50`），
  重启必然换 id —— 这一半在代码上是确定的；
  但"robotd 是否接受新 publisher 的首帧、不被旧 sequence 永久拒绝"属接收侧逻辑，无 robotd 无法验证。

**T07 非法/过期数据防护 → 阻塞**
- 构造非法帧（错误 schema、缺字段、NaN/Inf、超大陀螺仪、四元数范数错误、错误 publisher、
  sequence 倒退）需要 robotd 作为接收端才能观察"是否被拒绝、是否进入控制环"。
- 本机无 socket 文件可写入，也无接收进程，本条完全无法执行。

**T08 重复实例和 socket 安全 → 阻塞**
- 步骤 1（重复启动 robotd）：二进制不存在。
- 步骤 2（socket 路径替换为普通文件后启动）：robotd 不存在，无法观察其是否覆盖/删除普通文件。
- 步骤 3（清理残留 socket 后启动）：无 socket、无进程。

**T09 无权限用户访问 → 前置不通过**
- 执行单要求"确认 `zelux` 用户属于 `robot` 组"。实测：

```
$ id zelux
uid=1001(zelux) gid=1001(zelux) groups=1001(zelux),20(dialout),29(audio),44(video),
46(plugdev),102(netdev),987(gpio),986(i2c),985(spi)
$ getent group robot zelux
zelux:x:1001:
```

`robot` 组**在本机不存在**，`zelux` 也不属于任何 `robot` 组。
因此 T01 步骤 2 期望的 `root:robot 0660` 在本机当前状态下不可能成立。
- socket 不存在，合法/非法发布端的发送测试都无法进行。

**T10 robotd init 与总线互斥 → 阻塞**
- `robotd init` 不是可用命令，robotd 也未运行，"
运行时被锁拒绝 / 停止时可独占总线"两个分支都无法构造。

### 8.4 阻塞根因与解除条件

10 条用例卡在同一组前置条件上，按修复顺序排列：

1. **robotd 未部署**（影响 T01–T03、T05–T08、T10，共 8 条）
   缺可执行文件、缺 `/etc/robotd/robotd.toml`、缺 systemd 单元。
   解除：构建 aarch64 产物并安装。**不建议在板端直接编译**——本机 963MiB 内存（可用 760MiB）、
   缺 pkg-config/cmake/protoc 与 dbus/asound/udev 开发库，工作区 20+ crate 含 `ort`，
   大概率 OOM 或耗时不可控；应在 x86 主机交叉编译后部署。
2. **无 IMU 硬件**（影响 T01、T04、T06 的发布侧）
   解除：在 i2c-3 接入应答地址 0x68/0x69/0x6B/0x6A 的 IMU 模块，或提供 IIO 设备。
3. **无舵机总线**（影响 T01、T02、T03、T10）
   解除：启用 uart2 并释放 `console=ttyS2`，或接 USB Dynamixel 适配器。
   注意这与 `DEPLOY-ISSUES.md` 中 P0-5 是同一根因，属镜像/板级 bring-up 遗留。
4. **无 `robot` 组**（影响 T09，且是 T01 权限断言的前提）
   解除：robotd 安装时创建 `robot` 组并把 `zelux` 加入，socket 以 `root:robot 0660` 创建。
5. **无 Rust 工具链**（影响第 3 节的 4 条 Rust 命令）
   解除：在 Linux CI 或具备工具链的主机上补齐 `cargo fmt --check` 与三个 crate 的 `cargo test`。

上述 1–4 任一未解决，T01–T10 都无法产出有意义结论；当前状态**不能用于判断解耦是否达标**。

### 8.5 建议的执行顺序（解除阻塞后）

1. 先修 8.2 D1，重跑 `test_hal_selection.py`，确认 3 passed；
2. 部署 robotd + 创建 `robot` 组 + 安装 `robotd.toml`，先跑 T08（不需要运动，风险最低）；
3. 接 IMU 但不接舵机，跑 T05、T06、T07（纯数据链路，不涉及运动安全）；
4. 接舵机后按 T01 → T02 → T03 → T04 → T09 → T10 顺序执行，T10 涉及总线独占，放最后。


## 9. 本轮执行证据（2026-11-15 22:45–22:51，GMT+8）

### 9.1 前置条件与测试边界

- 真实 robotd 为 inactive/disabled，这是此前 UART2 初始化后保留的安全状态，本轮没有恢复它。
- Agent、风扇服务 active。配置位于 /etc/robot/robotd.toml（不是旧记录中的 /etc/robotd）。source=ipc，socket=/run/zelux/robotd-imu.sock，max_age_ms=100。
- /dev/ttyS2 已存在，root:zelux 0660。测试结束 fuser 无占用，pgrep 无 robotd 测试残留进程。
- Agent 本次启动日志：没找到 IMU（i2c-3 上没有 0x6B/0x68/0x69/0x6A 应答，IIO 也没有）。这是当前驱动未探测到，不等于电气检查证明物理硬件不存在。
- 同一启动周期的 22:40–22:42 Studio 日志报告接管 15/15 在线后交还；本轮没有重新扫描或动作。
- --fake 强制使用合成 embedded IMU，不创建真实 IPC IMU 接收端。因此其结果仅用于通用 daemon socket/锁，不冒充 T05–T07。
- 未上力、HOME、运动或真实 embedded 切换；未改变生产配置、永久权限或服务启用状态。创建并清理了隔离临时测试目录；compileall 产生 Python 字节码缓存。

### 9.2 第 3 节自动化测试

目标板 venv 未安装 pytest/jsonschema，直接执行要求的命令返回 No module named pytest/jsonschema。未安装额外依赖；改用标准库直接调用无 fixture 的原测试函数，并运行原 unittest 模块。以下不是 pytest 命令执行成功的记录：

| 检查 | 结果 |
|---|---|
| 已部署 test_imu_publish 的 3 个原函数 | 3/3 通过 |
| 已部署 test_hal_selection 原 unittest | 1 通过，2 失败 |
| 本机 test_imu_contract.py 原函数在目标板执行 | 2/2 通过，调用已部署 Drivers |
| 本机 test_catalog.py 原函数在目标板执行 | 3/3 通过，调用已部署 Drivers |
| 目标板 compileall -q 已部署 zeluxd | rc=0 |
| 本机协议 tools/validate.py | 6 个 JSON 文件基础解析通过；该工具不是完整 Schema 校验器 |
| 完整 JSON Schema 示例校验 | 阻塞，目标板缺 jsonschema；JSON 解析不能代替 Schema 验收 |
| Rust fmt / duck-control、robotd-params、robotd cargo test | 未执行，目标板无 cargo/rustc，本机无可用 WSL Rust 环境 |

HAL 失败断言：期望 zelux.tof.custom，得到 zelux.tof.st-vl53l5cx；期望“启动失败”，实际是“构造失败：未知驱动 zelux.tof.broken”。这是注入工厂与独立 catalog 的集成回归失败，不能外推为所有已注册 IMU 型号都无法切换。T04 实际切换仍未执行。

### 9.3 隔离验证方法和结果

使用设备现有 /usr/local/bin/robotd（0.14.1），由 microduck 运行 --fake --no-policy，指定新建临时目录下的独立 --socket，不访问 /dev/ttyS2。完成后退出测试进程并清理目录。

- T08.1：同端点第二个进程退出 1，日志 another robotd owns this socket。
- T08.2：端点预置普通文件 preserve-me，进程退出 1，日志 refusing to bind over it；内容未变。
- T08.3：预置已关闭的 Unix stream socket，fake daemon 成功重新监听。只证明控制 socket 恢复，不证明 datagram IMU socket。
- T10 锁分支：fake daemon 持同一端点锁时，init 退出 1，日志 another robotd owns this socket；还指定了不存在的串口，避免失败时触达真实总线。没有执行无人持锁的 init。
- T06 发布侧：两次独立 Python 子进程以 zelux 运行已部署 ImuPublisher，使用明确标记 SYNTHETIC 的合成 HAL，向隔离 Python 接收器各发送 15 帧；两次 publisher_id 不同，每次 sequence 都是 0–14。未向生产 socket 注入数据。
- T09 权限旁证：隔离目录 root:robot 0750，datagram socket root:robot 0660；zelux 发布成功；nobody 发送失败，PermissionError errno 13。未放宽为 0777，生产 socket 不存在，不能替代实际部署验收。

首次 T08 fixture 由 root 创建且未给 microduck 写目录权限，实例 bind 前 Permission denied；修正临时目录属主后重跑通过。这是测试夹具问题，未修改产品代码。原始输出见同目录 imu-decoupling-validation-20261115-evidence.md。

### 9.4 结束状态与后续前提

Agent/风扇 active；robotd inactive/disabled；真实 IMU socket 不存在；/run/zelux 为 root:zelux 0750。二进制 SHA-256：df54a2f62c08f4209433b1a7602dd29428adabbaa463cc161193677c87e725b4。版本号/哈希不证明其包含本机尚未提交的 Rust 修改。

下一轮先恢复有效物理 IMU 读数，确认 robotd 与 Feetech 协议配置一致、总线独占、现场支撑与断电看护；再执行真实 T01/T02、T05–T07、生产 socket 的 T09。T03 需旧 ID 200 硬件；T10 停机动作分支需满足动作安全条件。按本执行单要求，本轮不修改产品实现来修绿结果。
