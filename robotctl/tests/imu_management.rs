//! IMU management reads locally and sends all writes to robotd over its protected socket.
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::net::UnixListener,
    process::{Command, Output},
    thread,
};

fn invoke(file: &std::path::Path, socket: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_robotctl"))
        .arg("--robot-socket")
        .arg(socket)
        .arg("imu-config")
        .arg("--file")
        .arg(file)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn json_catalog_is_readable_without_socket_or_write_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("robotd.toml");
    let socket = dir.path().join("absent.sock");
    std::fs::write(&file, "[bus]\nprotocol = \"feetech-sts\"\n").unwrap();
    let report = invoke(&file, &socket, &[]);
    assert!(report.status.success(), "{:?}", report);
    let catalog: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(catalog["schema"], "zelux.robotd-imu-management/v1");
    assert_eq!(catalog["config"]["protocol"], "feetech-sts");
}

#[test]
fn selecting_a_model_is_sent_to_robotd_and_never_writes_the_local_file() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("robotd.toml");
    let socket = dir.path().join("robotd.sock");
    std::fs::write(&file, "[bus]\nprotocol = \"feetech-sts\"\n").unwrap();
    let original = std::fs::read(&file).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        stream
            .set_write_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let hello: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(hello["method"], "hello");
        writeln!(stream, "{}", serde_json::json!({
            "jsonrpc": "2.0", "id": hello["id"],
            "result": {"api_version": duck_ipc_proto::API_VERSION, "daemon_version": null, "revision": null}
        })).unwrap();

        line.clear();
        reader.read_line(&mut line).unwrap();
        let request: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], "robot.imu.configure");
        assert_eq!(request["params"]["model"], "qingmu-a1");
        assert_eq!(request["params"]["expected"], "rev-1");
        writeln!(
            stream,
            "{}",
            serde_json::json!({
                "jsonrpc": "2.0", "id": request["id"],
                "result": {"accepted": true, "management": {
                    "schema": "zelux.robotd-imu-management/v1", "revision": "rev-2"
                }}
            })
        )
        .unwrap();
    });

    let output = invoke(
        &file,
        &socket,
        &["--model", "qingmu-a1", "--expected", "rev-1"],
    );
    server.join().unwrap();
    assert!(output.status.success(), "{:?}", output);
    let saved: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(saved["revision"], "rev-2");
    assert_eq!(std::fs::read(&file).unwrap(), original);
}

#[test]
fn restart_is_a_confirmed_revision_checked_robotd_request() {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("robotd.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let hello: serde_json::Value = serde_json::from_str(&line).unwrap();
        writeln!(stream, "{}", serde_json::json!({"jsonrpc":"2.0","id":hello["id"],
            "result":{"api_version":duck_ipc_proto::API_VERSION,"daemon_version":null,"revision":null}})).unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap();
        let request: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(request["method"], "robot.imu.restart");
        assert_eq!(request["params"]["expected"], "saved-revision");
        assert_eq!(request["params"]["confirmed"], true);
        writeln!(
            stream,
            "{}",
            serde_json::json!({"jsonrpc":"2.0","id":request["id"],
            "result":{"accepted":true}})
        )
        .unwrap();
    });
    let output = Command::new(env!("CARGO_BIN_EXE_robotctl"))
        .arg("--robot-socket")
        .arg(&socket)
        .arg("imu-restart")
        .args(["--expected", "saved-revision"])
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(output.status.success(), "{:?}", output);
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["accepted"], true);
}
