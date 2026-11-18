# 部署问题修复：配置写入、Linux 脚本与 ONNX Runtime

日期：2026-11-18。状态：本地代码已修复，设备端部署验收待执行。

## 1. 保持解耦的边界

- Studio 只展示状态、收集型号/标定输入和用户确认，不写系统配置。
- Device Agent 保持 zelux 身份、NoNewPrivileges 和 ProtectSystem=full；不提权、不开放 /etc/robot 写权限。
- robotd 拥有自己的配置文件和锁，由 robotd 负责校验、加锁、原子保存。
- 运行库由 robotd 部署/更新链路管理，不塞进 IMU 驱动或 Studio。
- 保存只产生待重启配置；不重启、不切换实时控制源、不发送运动命令。

## 2. IMU 保存失败的根因和修复

Agent 过去直接执行 robotctl imu-config 本地写入模式。Agent 的 systemd 文件系统隔离将 /etc 设为只读，因此创建 robotd.lock 就失败；这不是锁文件损坏，也不能靠删除锁解决。

新调用链：Studio → Device Agent → robotctl → 本机 robotd Unix socket → 原有配置编辑器。

协议新增 robot.imu.configure，IPC API 从 32 升为 33。请求提供 config_path、expected 配置修订，以及 model 或 mount_correction_wxyz 二选一。

安全和一致性：

- 只接受 robotd 实际使用的配置路径，不接受任意文件写入；模拟进程拒绝修改真实配置。
- 保留现有文件锁、修订冲突检查、型号/总线兼容性检查、四元数校验、启动配置校验和原子写入。
- 返回保存后的管理快照；不让旧进程的运行状态冒充新配置已经生效。
- BLE/WebRTC 不开放此方法，Agent 通过已有受权限保护的本机 socket 调用。
- 失败不回退到 Agent 本地写文件。robotd 不在线或版本不匹配时应明确失败。

qingmu-a1 继续使用飞特协议及其现有安装/机体对齐逻辑；不借此次修复改变坐标系。qingmu-b1 仍维持未验证状态，不能冒充可用驱动。

## 3. Windows 换行与打包

Device Image 增加 Linux 脚本/overlay 的 LF 属性、CI 编码检查，并在打包、安装、两种镜像生成路径中规范化仓库自有启动入口。不扫描或改写设备其他程序。

robotd 的发布归档器对带 shebang 的文本入口（含生成的 hook）统一 CRLF → LF，保持程序二进制字节不变；补充归档回归测试。脚本及 hook 设置 LF 属性。

这解决的是后续构建产物；不会自动修复已经部署到设备上的旧副本。

## 4. ONNX Runtime 缺失

硬件可读与策略可运行是不同能力。缺少 libonnxruntime.so 时继续报告策略不可用/健康异常，不伪装成功。根据此次报告，无法仅凭症状断定是绕过更新 hook、安装遗漏还是动态库搜索路径问题。

- 原有发布 preinstall 保留运行库门禁；修复悬空软链接可能被误识别为已安装的问题，并确保目标目录存在。
- setup-board.sh 新增 --onnxruntime-only，只安装/修复已固定版本的运行库、刷新动态库缓存，验证绝对路径及系统 soname 均可加载。
- 此入口不改串口、网络、蓝牙、音频、启动 overlay 或 IMU 配置，不重启服务。

使用更新后的 robotd 源码，将机器人可靠支撑并停止运动后，在设备上的该源码根目录执行：

    sudo sh scripts/setup-board.sh --onnxruntime-only

不要为了修运行库无参数重跑完整 setup-board.sh。修复后另行安排安全重启和健康核验；不能把安装命令成功等同于策略已验收。

## 5. 需要部署什么

1. 同一新构建中的 robotd 与 robotctl（API 33）；不要只换其中一个。完整 daemon 发布应同时携带重新构建的服务端组件。
2. 更新 Device Agent 的 robotd_imu 桥接代码，以传递正确 socket 参数。
3. 重新生成 Device Image/安装包，使新启动脚本格式检查生效。
4. 设备修复 ONNX Runtime 后，单独确认策略健康。仅 git pull 不等于重新构建、安装或进程已更新。

此次未连接设备、未重启服务、未发送运动命令，也未执行 Git 提交。

## 6. 验证记录和限制

- Device Agent：134 项 Python 单元测试通过，包括型号/标定保存传递指定 socket、保存后待重启且不自动重启。
- robotd 与 robotctl：aarch64-unknown-linux-gnu cargo check 通过。
- 协议测试与 robotctl IMU 集成测试：aarch64 目标编译检查通过，尚未在 Linux 执行。
- 部署脚本逐文件 bash -n 通过；Device Image 的 LF/BOM 检查（24 文件）和组件锁验证通过。
- 新增 robotd 配置写入测试覆盖保存、过期修订拒绝、错误路径及二选一校验；新增归档脚本换行/二进制不变测试。
- 本机 Windows 无 Linux 测试运行环境；完整 Rust 测试仍待 Linux CI/构建机执行。robotd/xtask 测试交叉构建还受缺少 aarch64-linux-gnu-gcc 限制，btd/mediad 全量构建受 GLib 交叉 sysroot 限制。不能将 cargo check 记为设备测试通过。

## 7. 设备非运动验收

先可靠机械支撑、保持策略关闭，不发送运动指令。记录部署包、进程路径及版本。

1. 首次启动和重启后确认启动入口不再报解释器/换行错误，robotd 与 Agent 服务可用。
2. 以实际 Agent 服务身份及其现有沙箱，从 Studio 保存当前 qingmu-a1；不能出现只读文件系统错误。
3. 查看返回新修订/待重启标记；确认保存没有改变进程实例、不会触发运动。
4. 在现有标定流程中保存有效结果，同样核对文件持久化、修订和待重启状态。
5. 用旧修订再次保存应拒绝，文件不变；错误路径、未支持型号、非法四元数也应拒绝。
6. 用户明确确认并满足现有安全条件后重启，核对新进程应用正确型号与标定；核对飞特总线设置未被修改。
7. 运行库修复后检查 robotctl health：分别记录控制环、总线、IMU、策略状态；若仍 unhealthy，保留原始具体原因，不直接判定 IMU 故障。
8. 单独检查总控实时设备状态与标定待办分离（此前改动），不把“未标定”显示成“未接线”。

任一步失败，保存日志和配置差异，暂停后续启动策略；本轮验收不要求行走或站立动作测试。
