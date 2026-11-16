# IMU decoupling device test evidence

Device: 192.168.2.117. Date: 2026-11-15, GMT+8. Synthetic tests are explicitly scoped; no production IMU acceptance is claimed.

## Original test functions on deployed modules

```text
PASS imu_publish.test_control_readiness_does_not_depend_on_studio_calibration_wizard
PASS imu_publish.test_packet_preserves_units_and_driver_identity
PASS imu_publish.test_packet_requires_a_control_frame
IMU_PUBLISH_DIRECT 3
HAL_SELECTION_RC 1  tof 驱动 zelux.tof.custom 构造失败（'未知驱动：zelux.tof.custom'），回退默认驱动
F.tof 驱动 zelux.tof.broken 构造失败（'未知驱动：zelux.tof.broken'），回退默认驱动
F
======================================================================
FAIL: test_selected_factory_controls_each_hal_part (__main__.HalSelectionTests.test_selected_factory_controls_each_hal_part)
----------------------------------------------------------------------
Traceback (most recent call last):
  File "/opt/zelux/releases/1.0.0/software/zeluxd/tests/test_hal_selection.py", line 52, in test_selected_factory_controls_each_hal_part
    self.assertEqual(hal.tof.driver_id, "zelux.tof.custom")
    ~~~~~~~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
AssertionError: 'zelux.tof.st-vl53l5cx' != 'zelux.tof.custom'
- zelux.tof.st-vl53l5cx
+ zelux.tof.custom


======================================================================
FAIL: test_start_failure_falls_back_only_failed_device (__main__.HalSelectionTests.test_start_failure_falls_back_only_failed_device)
----------------------------------------------------------------------
Traceback (most recent call last):
  File "/opt/zelux/releases/1.0.0/software/zeluxd/tests/test_hal_selection.py", line 71, in test_start_failure_falls_back_only_failed_device
    asyncio.run(run())
    ~~~~~~~~~~~^^^^^^^
  File "/usr/lib/python3.13/asyncio/runners.py", line 195, in run
    return runner.run(main)
           ~~~~~~~~~~^^^^^^
  File "/usr/lib/python3.13/asyncio/runners.py", line 118, in run
    return self._loop.run_until_complete(task)
           ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~^^^^^^
  File "/usr/lib/python3.13/asyncio/base_events.py", line 725, in run_until_complete
    return future.result()
           ~~~~~~~~~~~~~^^
  File "/opt/zelux/releases/1.0.0/software/zeluxd/tests/test_hal_selection.py", line 66, in run
    self.assertIn("启动失败", hal.driver_report["tof"]["error"])
    ~~~~~~~~~~~~~^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
AssertionError: '启动失败' not found in "构造失败：'未知驱动：zelux.tof.broken'"

----------------------------------------------------------------------
Ran 3 tests in 0.039s

FAILED (failures=2)

PASS imu_contract.test_quaternion_stays_normalized
PASS imu_contract.test_upright_control_orientation_matches_robotd_frame
PASS catalog.test_all_device_kinds_have_a_model_adapter
PASS catalog.test_descriptors_are_serializable_and_stable
PASS catalog.test_factories_satisfy_complete_contracts
PYTHON 3.13.5 (main, Aug 10 2026, 12:06:59) [GCC 14.2.0]

```

## Isolated fake robotd socket and init lock checks

```text
duplicate_rc 1 message 2026-11-15T14:50:07.291237Z ERROR robotd: cannot claim robot IPC socket path=/tmp/zelux-imu-t08-8na9v4jz/primary.sock error=another robotd owns this socket
init_while_running_rc 1 message 2026-11-15T14:50:07.330744Z ERROR robotd: cannot claim robot IPC socket path=/tmp/zelux-imu-t08-8na9v4jz/primary.sock error=another robotd owns this socket
regular_rc 1 file preserve-me message 2026-11-15T14:50:09.436799Z ERROR robotd: cannot claim robot IPC socket path=/tmp/zelux-imu-t08-8na9v4jz/regular.sock error=the socket path exists and is not a socket; refusing to bind over it
stale_socket_mode 0o755
PASS ['duplicate instance refused', 'init refused while fake daemon holds same lock', 'regular file preserved', 'stale socket reclaimed']

```

The immediate stale_socket_mode sample above races asynchronous chmod after bind; it is not a final permission assertion. T09 permissions were measured on the explicit 0660 datagram fixture below, not this control socket.

## Synthetic HAL publisher / isolated Python receiver

```text
publisher_process 0 frames 15 first_sequence 0 publisher_id 660fb8b3f5d84947b8dc798d2fe28d81
publisher_process 1 frames 15 first_sequence 0 publisher_id 6031585c7dfb46069c61f35f319aefc1
publisher restart metadata PASS (synthetic HAL, Python receiver; NOT robotd receiver)
nobody_rc 1 error PermissionError: [Errno 13] Permission denied
fixture permissions PASS root:robot 0660; zelux allowed, nobody denied; actual robotd IMU socket NOT verified
active
active
inactive
disabled

```

## Final device state

```text
CMD date -Is RC 0 2026-11-15T22:51:45+08:00 
CMD /usr/local/bin/robotd --version RC 0 robotd 0.14.1 
CMD systemctl is-active zeluxd zelux-fand robotd RC 0 active
active
inactive 
CMD systemctl is-enabled robotd RC 1 disabled 
CMD id zelux RC 0 uid=1001(zelux) gid=1001(zelux) groups=1001(zelux),20(dialout),29(audio),44(video),46(plugdev),102(netdev),987(gpio),986(i2c),985(spi),984(robot) 
CMD fuser -v /dev/ttyS2 RC 1  
CMD pgrep -a robotd RC 1  
CMD stat -c "%a %U:%G %n" /run/zelux /dev/ttyS2 RC 0 750 root:zelux /run/zelux
660 root:zelux /dev/ttyS2 
/opt/zelux/current/software/zeluxd/__init__.py """ZELUX Duck Studio —— 机械鸭主控 Web 控制台后端。"""

__version__ = "1.4.0"

/opt/zelux/current/software/zelux_drivers/__init__.py """ZELUX 可插拔设备驱动及内置型号目录。"""

__version__ = "1.2.0"

from .catalog import create, descriptor, descriptors, register
from .contracts import (C
/opt/zelux/protocol/VERSION 1.2.0

IMU_CONFIG 
source = "ipc"
socket = "/run/zelux/robotd-imu.sock"
max_age_ms = 100

IMU_SOCKET_EXISTS False
ROBOTD_SHA256 df54a2f62c08f4209433b1a7602dd29428adabbaa463cc161193677c87e725b4

```

