# IMU Unified Acceptance Test Evidence

**Test Host:** Ubuntu 24.04 x86_64 (192.168.2.110)
**Execution Date:** 2026-11-16
**Test Type:** Software validation + ARM64 cross-build + Physical target read-only check

---

## First-Gate Verification Results (x86_64 Host)

### robotd

**Script:** `scripts/validate-imu.sh`
**Result:** PASS
**Exit Code:** 0

**Evidence Location:** `~/imu-acceptance/zelux-robotd/target/imu-validation/run-nqt2rrLN`

**Test Categories:**
- test/strict
- clippy/build

**Hardware Tests:** None (x86 host, no physical IMU)

**x86_64 Binary Hashes:**
| Binary | SHA256 Hash |
|--------|-------------|
| robotd | `405363178d53fa3769a62aa73daf62a1919438c1784c396047472326becb73ba` |
| robotctl | `e150f7af79fcf82175a55816a324a9aeba6d1c8cccda0e0fad15819ae6c7b6b2` |

### agent

**Targeted Unittest:** 42 PASS
**Firmware Unittest:** 7 PASS

### studio

**State Assertions:** 9 PASS
**DOM Integration:** PASS
**Package:** `zelux-studio-1.2.0.zip` PASS

### protocol

**Schema/Examples Validation:** PASS (10 files, including IMU samples)
**Package:** `zelux-protocol-1.3.0.zip` PASS

---

## Source Archives

| Stage | SHA256 Hash |
|-------|-------------|
| Initial upload (without .git) | `acdf827ad2bbca9c0185b246eee0c2eeeca8ece7167873a9f25fce3ef71413af` |
| Final upload | `65b5cd31deadfb0c22ef3cea73706f63cd2071577799a6bc019ecb5424544d64` |

## Repository HEADs

| Repository | Commit SHA |
|------------|------------|
| robotd | `22fcca3b0d3dfe390707e9668200f695197c2059` |
| agent | `e054c6037699f8b57b9495f27f2cf3abbb8d0d2a` |
| studio | `5e74d8e398b55592d3c363342c6d2da45451feb9` |
| protocol | `d2296dfb2a0d14a0675f34c39fab3cedfaccf47f` |
| drivers | `d36471e003f0b77c7a9d8262c4e592239d171890` |

---

## Continuation Results (2026-11-16)

### ARM64 Cross-Build

**Command:** `cargo board --bins`
**Build Host:** 192.168.2.110
**Sysroot:** Debian Trixie ARM64 (from `scripts/cross-sysroot.sh`)
**Result:** PASS
**Exit Code:** 0

**Artifact Verification:**
| Binary | SHA256 Hash | Format |
|--------|-------------|--------|
| robotd | `de7a798bb01be369f8406f485188b0fe0bdd5135485ee332117e815a587fa2c7` | AArch64 ELF |
| robotctl | `58620b7e2d981e25943b9926b3b128a3c7323d6b2d2f26d83bb1116e6f528eea` | AArch64 ELF |

**GLIBC Compatibility:**
- Max referenced GLIBC symbol: 2.30
- Build target pinned: 2.31
- Status: Compatible with Trixie ARM64 sysroot

**Build Warnings (Non-Blocking):**
- OBSERVED: mediad deprecated `Atomic::fetch_update` warning (pre-existing, unrelated to IMU changes)

---

### Physical Target Read-Only Check (192.168.2.117)

**Result:** OBSERVED (read-only inventory, no modifications)

**Current Installation:**
- `/opt/robot/daemon/current` -> `/opt/robot/daemon/releases/0.14.1`
- Installed robotd SHA256: `df54a2f62c08f4209433b1a7602dd29428adabbaa463cc161193677c87e725b4`
- robotd.service: inactive, disabled
- `/run/robotd.sock`: not present
- Install tree: contains robotd, no robotctl

**Candidate Artifact Verification:**
- AArch64 robotctl candidate copied to `/home/microduck/robotctl-imu-candidate` (not installed)
- Candidate SHA256: matches build artifact `58620b7e2d981e25943b9926b3b128a3c7323d6b2d2f26d83bb1116e6f528eea`
- Read-only `imu-config --file /etc/robot/robotd.toml`: succeeded

**Current Configuration (read-only inspection):**
| Field | Value |
|-------|-------|
| protocol | `dynamixel2` |
| source | `ipc` |
| model | `null` |
| max_age_ms | `100` |

**IMU Candidate Compatibility Assessment:**
| Candidate | Status | Reason |
|-----------|--------|--------|
| A1 | Incompatible | Requires Feetech protocol |
| B1 | Unsupported | Not supported by this configuration |

---

### Live HTTP Read-Only Check (192.168.2.117)

**Health Endpoint:** `http://192.168.2.117:8000/api/health`
- Result: PASS
- Response: `{"ok":true,"version":"1.4.0","mode":"radxa"}`

**Root Endpoint:** `http://192.168.2.117:8000/`
- Result: PASS
- Response: 200, Studio HTML served

**IMU Management Endpoint:** `http://192.168.2.117:8000/api/imu/management`
- Result: 404 Not Found
- OBSERVED: Deployed Agent/API does not expose new IMU route

---

## Blocking Issues

The following findings **block** further validation:

### U: Live UI / Status Evaluation
**BLOCKED BY:** Deployed Agent/API version 1.4.0 does not expose `/api/imu/management` endpoint. UI candidate cannot be tested against deployed target without correct candidate service layout.

### C: Hardware Tests (All)
**BLOCKED BY:**
1. Incorrect candidate service layout (no robotctl in install tree, Agent missing IMU route)
2. **User onsite safety confirmation required** before any:
   - Service restarts
   - Serial bus access
   - Deployment/current symlink changes

---

## Actions NOT Performed

- No services restarted
- No serial bus opened
- No deployment/current symlink changes
- No configuration changes recommended or applied

---

## Scope Clarification

### Validated
- x86_64 host build and tests (PASS)
- ARM64 cross-build (PASS)
- Physical target read-only inventory (OBSERVED)
- Live HTTP health check (PASS)
- IMU route availability check (404 - OBSERVED)

### Not Validated (Blocked)
- ARM64 deployment to target
- Live UI candidate test against target
- Physical IMU hardware
- Real serial bus access
- Safe configuration tests
- Safe restart tests
- Action tests

---

## Notes

- All x86_64 first-gate tests preserved above
- ARM64 build artifacts validated as AArch64 ELF with correct hashes
- Physical target 192.168.2.117 inspected read-only; no modifications
- Deployed Agent version 1.4.0 does not have IMU management endpoint
- A1 candidate incompatible with current Feetech requirement; B1 unsupported
- No claims beyond documented evidence
- User onsite safety confirmation required before any hardware/service actions

## 真机流订阅追加记录（2026-11-16）

在用户确认安全条件并授权后，用与前次相同哈希的候选 robotd/robotctl、临时 A1 + Feetech 配置和私有 socket 复测。`robotctl monitor --hz 1 --json` 是现有只读 IPC 订阅，可读取每帧 `imu.gyro`（rad/s）与 `imu.quat`，不需新增接口或重编译；该状态流不含加速度原始量与传感器采样计数。

采集 6 帧、约 5 秒基线。记录中 `fallen=true`；gyro 各分量约在 -0.0011 至 0.0138 rad/s，而四元数与派生 gravity 从 `[0.691, 0.709, -0.144]` 连续变化到 `[0.122, -0.977, -0.175]`。测试前未要求操作者转动设备，故不能假定这是静止基线；也不能据此判定轴向通过。该结果表现为姿态/角速度观测不一致，列为异常待查。立即停止候选进程，未做转轴操作、故障注入或 ID 200 写入。候选进程及私有 socket 已清理；正式 robotd 服务仍 inactive/disabled；生产配置哈希保持 `0b54e2f79a6e84b19f1298713dcd01685d1b607addb9a66b5454e260d98f2cf2`。

**结论：停止进一步物理动作测试，先确认采样期间设备是否被移动并定位姿态与 gyro 不一致原因。** 健康 ready 不能覆盖此异常。后续转轴测试需在确认稳定基线后逐轴进行；本接口可验证 gyro 响应，不能验证加速度/重力原始方向、采样计数冻结或断线恢复。

## 静止复测追加（用户确认期间未移动，2026-11-16）

按用户要求继续观察，不做手动转动。相同隔离候选实例累计完成一段 20.9 秒静止观测（42 帧、2 Hz）和一段 29.9 秒状态流采集（142 帧、5 Hz）。静止重复数据中，派生重力三个分量范围分别为 X `[-0.79, 0.94]`、Y `[-0.90, 0.86]`、Z `[-0.61, -0.33]`；`fallen` 在 42 帧中 true 18 次、false 24 次。gyro 范围为 X `[-0.0064, 0.0032]`、Y `[0.0043, 0.0149]`、Z `[-0.0096, -0.0011]` rad/s。相邻 2 Hz 姿态四元数角差约 12.8–14.3°，同一间隔按 gyro 积分的转角仅约 0.23–0.47°。机器人未被操作者移动，因此姿态流与 gyro 明显不一致可复现；这不是“轴向验收通过”。

复测启动时有 1 次 `motor sync_read: Parsing error`（consecutive=1），随后 `imu-status` 显示健康、IMU ready、stale=0、bus errors=0，控制环约 50 Hz、missed=0；电机最高温度读数 58°C。尚不能确定解析错误根因。候选进程已终止，私有 socket 与总线占用已释放；正式 robotd 仍 inactive/disabled，正式配置哈希未变。30 秒采样窗口曾提示操作者进行小幅 X 轴动作，但尚未收到是否执行及舵机阻力的确认，因此不把该窗口记为有效转轴样本。

**当前判定：真实传感器数据路径已连通且 ready 可读；静止姿态一致性验收失败/待定位，轴向符号测试未完成。** 后续不能把 `ready=true` 或状态接口通畅当作正确性证明。应优先获取原始 15 字节帧（仅通过 robotd/BusIo 所有者实现），复核实际帧内容、寄存器语义、四元数和角速度的时间对应及 shared-bus 解析错误；不得用额外串口工具并行读取来绕过单一总线所有者。

## 舵机卸力记录（2026-11-16）

后续现场反馈：虽然该卸力命令返回成功，用户仍感到关节卡住；不可将下文执行日志视为已确认物理卸力。用户随后重启、固定台架，并确认关节卸力。

应用户要求，短暂启动私有候选 robotd 后执行官方命令 `robotctl robot relax --yes`。CLI 返回 `torque off`，robotd 日志确认 `safety: torque on=false` 及 `robot.relax: torque off`。随后已停止候选进程并确认验证 socket 移除、`/dev/ttyS2` 无占用者；正式 `robotd.service` 仍 inactive/disabled，生产配置 SHA256 未变。未单独读取每个舵机的 torque-enable 寄存器回读，因此证据是命令成功与 daemon 执行日志，而非逐舵机寄存器核验。安装台架时仍应托住机器人，避免松脱或坠落。

## 台架纯读取诊断（2026-11-16，重启后）

新增 `duck-control/examples/imu_readonly.rs`，复用 rustypot Feetech 控制器与原 ImuDxl 解码器，不运行 robotd/control loop，不调用扭矩、目标位置、增益或 EEPROM 写方法。独占测试前检查 robotd 未运行、串口无占用；先逐只读取 15 个 JOINT_IDS 的寄存器 40，只有全部单字节为 0 才采样 ID 200。以 `cargo board -p duck-control --example imu_readonly` 在 Linux 构建成功；普通 cargo build 交叉链接因默认主机 cc 失败，未绕过，随后使用仓库 zigbuild 路径成功。

真机二进制 `/home/microduck/imu-readonly-bench` SHA256 为 `0f2ceeac6e56e13078bbc8939bb00e47c8b531eb02bb3f0c5b01c16c65c961a8`。运行退出码 0，记录 `/home/microduck/imu-readonly-static.jsonl` 共 215 行（15 行逐舵机 torque_enable=0，200 行 IMU 原始 15 字节帧及解码值）。200 次 ready=true，约 10.36 秒；序号有 158 个不同值，非冻结。起止重力分别 `[0.02675,-0.03414,-0.99906]` / `[0.02683,-0.03445,-0.99905]`，起止四元数接近；本段起止比较未复现此前大幅姿态摆动，不代表已定位原因或全面验证稳定性。未做三轴人工转动。

程序限次自动退出，串口释放；正式 robotd 未运行、正式配置 SHA256 未变。此例证明此次测试点 15 个舵机扭矩寄存器回读均为关闭，不保证之后被其他客户端操作仍保持关闭。下一步先通知用户并等其准备，再分轴采样；不能把无人工转动确认的窗口记为轴向测试通过。

## 台架 X 轴左倾采样（2026-11-16）

用户回复“已完成，向左倾”。先后两个纯读取窗口分别保存为 `/home/microduck/imu-readonly-axis-x.jsonl`、`/home/microduck/imu-readonly-axis-x-repeat.jsonl`，各含 1200 帧 IMU 和 90 次舵机扭矩回读；每窗口为 6 段采样，段内时间从零起算，未记录绝对时间，不能精确对齐用户动作时刻。第二窗口退出码 0、错误日志为空；1200/1200 ready=true，90 次 torque_enable 均为 0。结束后串口无占用，robotd inactive。

第二窗口派生重力范围：X `[0.02195,0.02838]`、Y `[-0.03552,-0.03077]`、Z `[-0.99925,-0.99897]`；第一窗口 Y 范围 `[-0.03528,-0.03134]`，同样未捕获明显侧倾姿态。第二窗口开头有短暂 gyro 响应，但姿态幅度不足以证明约 5° 机体左倾。当前不能区分动作未落入窗口、只移动了非 IMU 所在结构、实际幅度偏小或姿态数据响应异常。

判定：读取链路及测试点卸力回读通过；X 轴方向/符号验收未通过确认，仍待补测，不能把用户动作确认等同于传感器响应正确。后续需确认实际移动的是 IMU 所在机体，并使用更明确的采样时序。

### 保持左倾后采样

用户确认 IMU 所在躯干随整机左倾，并在回复“已保持左倾”后采样，排除仅凭短动作窗口推断。记录 `/home/microduck/imu-readonly-left-held.jsonl`：退出码 0，错误日志为空，200/200 ready=true，约 10.37 秒；15 颗舵机扭矩回读全部为 0。结束后串口无占用，robotd inactive。

派生 gravity 均值 `[0.04445,-0.30569,-0.95109]`；此前回正静止基线均值 `[0.02676,-0.03419,-0.99906]`，两者方向夹角约 15.88°，非外部量具测得角度。左倾段 Y 范围 `[-0.30748,-0.30449]`，姿态确有明显且保持稳定的变化；这说明前两窗口缺乏响应不能直接判为传感器不响应。尚待此次回正对照，未验证转动期间 gyro 符号、Y/Z 轴或 robotd 控制环异常根因。

当前约定机体 X 前、Y 左、Z 上；向左的空间方向为 +Y，不等于绕 +Y 轴转动。此处 gravity 为四元数派生的重力方向，不是加速度计原始比力；左倾时 gravity 的 Y 变负不能单独判为轴向错误。安装映射的完整正确性仍需对照回正与其他轴的受控动作。

### 左倾后回正对照

用户回复“已回正”后再采样 `/home/microduck/imu-readonly-upright-return.jsonl`：退出码 0，错误日志为空，200/200 ready=true，约 10.35 秒，15 颗舵机 torque_enable 全部为 0；结束后串口无占用，robotd inactive。gravity 均值 `[0.02521,-0.03393,-0.99911]`，与初始静止基线夹角约 0.090°，与保持左倾姿态夹角约 15.90°；Y 范围 `[-0.03411,-0.03366]`。

判定：本次纯读取路径的“回正基线→保持左倾→回正”重力方向响应及回归检查通过（角度为 IMU 推算，非量具校准）。不据此宣称全部 X 轴验收通过：转动过程 gyro 轴向/符号尚未验证，其他轴、正式 robotd 控制环此前异常和完整管理闭环仍需后续测试。

### 保持前倾后采样

用户回复“已保持前倾”后采样 `/home/microduck/imu-readonly-forward-held.jsonl`：退出码 0，错误日志为空，200/200 ready=true，约 10.34 秒；15 颗舵机 torque_enable 全部为 0，结束后串口无占用，robotd inactive。gravity 均值 `[-0.26269,-0.04286,-0.96393]`，与上一段回正均值的方向夹角约 16.69°（IMU 推算，非量具实测）；X 范围 `[-0.262701,-0.262684]`，Y 范围 `[-0.043139,-0.042609]`。前倾时重力投影主要向机体 -X 变化，与 X 向前、Z 向上的约定相符。

判定：保持前倾姿态方向响应正常，尚待前倾后回正对照；没有采集本次转动过程，不能据此判定 gyro Y 轴符号通过。

### 前倾后回正对照

用户回复“已回正”后采样 `/home/microduck/imu-readonly-forward-return.jsonl`：退出码 0，错误日志为空，200/200 ready=true，约 10.38 秒；15 颗舵机 torque_enable 全部为 0，结束后串口无占用，robotd inactive。gravity 均值 `[0.02990,-0.05142,-0.99823]`；距初始基线约 1.00°，距上次回正约 1.04°，距前倾保持姿态约 16.95°。本段 Y 范围 `[-0.051474,-0.051380]`，保持稳定。

判定：前倾姿态已明显回归接近原基线，但仍有约 1° 回正残差。无外部姿态量具，不能区分人工回位偏差与传感器偏差，也不据此宣称精度验收通过。前倾静态方向响应已观察确认；动态 gyro 符号、Z 轴响应及先前 robotd 控制环异常仍未验收。

### 水平左转及回转动态采样

用户确认台架允许水平转动，开启 18 段纯读取窗口并确认日志出现 capture_started 后提示动作；用户回复“水平左转并回正完成”。记录 `/home/microduck/imu-readonly-yaw-dynamic.jsonl`。窗口尚在运行时取回的快照含 1930 帧 IMU、150 次扭矩回读，全部 ready=true，扭矩回读全部为 0；以下为该快照分析，最终帧数与串口释放需结束后另核。

Z gyro 范围 `[-0.7460,+0.5268]` rad/s，X `[-0.1394,+0.1149]`、Y `[-0.2448,+0.2373]`。动作先出现持续正 Z，再出现持续负 Z，与用户先向左水平转、再反向回转的顺序一致；gravity Z 范围 `[-0.99982,-0.99821]`，机体保持接近直立。四元数 yaw（标准 wxyz 计算，仅作相对变化观察）动作前约 +1.2°，左转保持约 +91.9°，回转后约 -13.8°，相对起点残差约 -15°。动作区间段内梯形积分的绝对角位移以 Z 为主：约 196°，X 约 15°、Y 约 24°；跨段间隙未积分，不能作为精密角度标准。

判定：本次动态 Z 轴主响应与左右转符号观察符合预期，但实际动作角度需要用户确认。预先要求约 10°，数据却显示约 90° 左转；不能未经确认归因为人工转幅或传感器比例错误，也不能宣称角度精度/回正验收通过。动态 X/Y 轴符号仍未验证。

后续用户明确确认实际左转接近 90°，与上述约 90° 相对 yaw 变化吻合，解除“10° 动作却显示 90°”的疑点；不将本次记录作为比例异常证据。无外部角度量具，仍不作精度标定结论，回转残差约 15° 的原因尚不能区分人工回位偏差与传感器误差。

最终窗口正常结束，退出码 0；共 3870 行，包含 3600/3600 ready=true 的 IMU 帧和 270 次 torque_enable=0 回读（18 段、每段 15 颗）。错误日志仅有主动输出的 capture_started 标记，无其他错误。结束后串口空闲，robotd inactive。最后一段 yaw 范围约 `[-14.29855,-14.29306]`°，保持稳定。判定：本次纯读取路径水平左转/反向回转的 Z 轴主响应、角速度符号及约 90° 姿态变化的定性核对通过；不等于完整三轴、精度或 robotd 管理闭环验收通过。

### 左侧倾及回正动态采样

确认只读窗口启动后提示左侧倾并回正，用户回复“左侧倾并回正完成”。记录 `/home/microduck/imu-readonly-roll-dynamic.jsonl`；运行期间取回快照解析到 1471 帧 IMU、120 次舵机扭矩回读，全部 ready=true、torque_enable=0。文件行数查询比取回快照稍晚，不能将二者当成同一时刻计数；最终完成及串口释放尚待另核。

gyro 范围：X `[-0.26499,+0.20007]`、Y `[-0.08833,+0.11281]`、Z `[-0.03299,+0.08833]` rad/s。左倾阶段持续正 X，回正阶段持续负 X；gravity Y 随左倾从约 -0.032 降至最低 -0.411，回正恢复约 -0.030。动态响应以 X 轴为主，往返符号符合当前机体约定。相对本窗口动作前均值的最大倾角约 22.39°，回正末尾 100 帧均值与动作前均值方向差约 0.52°，均为 IMU 推算、非量具测量。指令请求 5～10°，实际转幅未由用户确认，不据此判定比例或精度。

判定：本次 X 轴侧倾/回正动态主响应与角速度符号定性检查通过；不等于角度精度验收。动态 Y 轴仍待测试，先前 robotd 控制环异常未定位。

### 前倾及回正动态采样与物理轴向判定更正

侧倾窗口已正常退出（exit 0），最终取回记录含 3600/3600 ready=true 帧及 270 次 torque_enable=0；确认其退出后才开启前倾窗口，无并行串口读取。

前倾窗口 `/home/microduck/imu-readonly-pitch-dynamic.jsonl` 在用户回复“前倾并回正完成”后取回快照：1157/1157 ready=true，90 次 torque_enable=0。gyro 范围 X `[-0.01383,+0.02341]`、Y `[-0.22987,+0.29479]`、Z `[-0.04576,+0.07769]` rad/s；前倾阶段持续负 Y，回正阶段持续正 Y。gravity X 前倾时变负，回正恢复；相对本窗口基线最大倾角约 22.79°，末尾 100 帧均值回正差约 0.153°。角度均由 IMU 推算，无量具校准。该快照不代表最终窗口已完成或串口已释放。

重要更正：上述响应说明 gyro 与姿态有主轴对应及往返响应，但不能直接认定物理符号符合既定机体坐标。若实际机体确实 X 向前、Y 向左、Z 向上且采用右手角速度，则向前俯应为正 Y，躯干顶端向左侧倾应为负 X；本次分别观察到负 Y、正 X，与此前口头“方向通过”的结论相反。此前静态前倾 gravity -X 被直接描述为符合预期，以及侧倾正 X 被直接标为物理方向通过，均应以此条更正：X/Y 物理符号待核，不能标最终通过。可能涉及实际安装相对机体朝向、配置映射、操作者动作定义或固件轴约定，尚未定位，不自动改符号或安装配置。Z 轴左转正/回转负的定性观察仍保留。需核对 IMU 安装朝向与实际机器人前方后再作完整坐标验收。

### 本轮人工动作采样收尾

前倾窗口已正常结束，exit 0；最终 3600/3600 ready=true，270 次 torque_enable=0，错误日志仅 capture_started 标记。已确认串口无占用，robotd inactive。本轮静态、左右侧倾、前倾、水平转向及回正人工动作采样结束，无需用户为结束采样继续操作。用户不提供安装照片；照片不是采样收尾的必要条件，X/Y 物理坐标符号疑点仍保留，不能改为通过。

整体状态：本轮三轴人工动作数据采集已完成，但统一验收尚未全部完成/通过。剩余包括 X/Y 物理方向疑点定位、此前 robotd 控制环静止姿态异常定位及复测、正式候选 Agent/Studio IMU 管理接口部署与保存/重启闭环、未执行的故障恢复与型号切换项目。不可将纯读取诊断通过替代这些验收项目。

## 共享总线混合只读排查（2026-11-16）

用户要求继续后，检查 FeetechIo::read/poll_imu_dxl、ImuDxl 解码器、SplitImuIo 和状态发布路径。诊断与 robotd 使用同一 ImuDxl 解码器，但先前诊断仅周期读 IMU，未模拟每周期先读全部舵机反馈的事务负载。因此扩展 `duck-control/examples/imu_readonly.rs`：可选 `--mixed-feedback`，1500 周期、目标 20 ms，每周期先读 15 颗舵机地址 56/长度 15，再读 ID 200；保存完整反馈块、IMU 原始帧、绝对 unix_ms 和 index；启动前及结束后各逐只检查扭矩关闭。保持只读，不启动 robotd，不调用舵机/EEPROM 写方法。该模式不模拟目标位置写入、控制环计算或完整生产服务负载，也不是生产 robotd 复测。

Linux 192.168.2.110 用仓库 cargo board 构建成功；在该主机执行 `cargo test -p duck-control imu_dxl --lib`，23 项通过、0 失败。真机独立诊断产物 `/home/microduck/imu-readonly-mixed-bench` SHA256 `03007c8e5a88e98ab20139b2004a5f8812d9eba6f34b07b4adc639e232a7666c`，没有替换正式服务程序。

真机 `/home/microduck/imu-readonly-mixed-static.jsonl`，exit 0、错误日志空：1500/1500 ready=true，每帧 15 个舵机反馈块均精确 15 字节；首尾共 30 次 torque_enable 回读全为 0。持续约 30.353 秒，帧间隔中位 20 ms、p99 21 ms、最小 17 ms、最大 23 ms（毫秒时间戳精度）。相对首帧最大四元数姿态角差约 0.537°；gravity 范围 X `[0.02037,0.02177]`、Y `[-0.03553,-0.02843]`、Z `[-0.99939,-0.99913]`；舵机反馈最高温度 43°C。没有复现此前静止姿态大幅连续变化，亦没有本次读事务错误。

结束后串口空闲，robotd inactive，正式配置 SHA256 仍为 `0b54e2f79a6e84b19f1298713dcd01685d1b607addb9a66b5454e260d98f2cf2`。本结果仅说明本轮混合只读负载正常，不能据此认定此前异常已修复或根因属于 robotd。没有旧异常时段的原始帧，无法离线重建其真实 IMU 输入。X/Y 方向问题仍未擅自翻转配置：当前固定 A1 映射假定 sensor +X down、+Y right、+Z back；实际动作与该映射在 X/Y 物理符号上尚待核对，Z 定性响应保留。

## 归档位置（2026-11-16）

本轮最终原始采样（11 个 JSONL 及 11 个配对 stderr）、两份真机 ARM64 只读诊断可执行文件已复制到 [`docs/test-results/imu-unified-acceptance-20261116/`](../test-results/imu-unified-acceptance-20261116/)。归档索引概述最终状态，`SHA256SUMS.txt` 提供每个原始文件与二进制的完整性校验。采样记录共 6,634,517 字节；设备未改正式配置，正式 robotd 保持停止，串口已空闲。归档不是全项验收通过的声明；开放问题详见索引和本记录前文。
