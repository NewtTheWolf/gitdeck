use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Sends an MCP `initialize` request over stdio and asserts the server responds
/// with a JSON-RPC result containing serverInfo. Proves the binary speaks MCP.
#[test]
fn server_responds_to_initialize() {
    let db = tempfile::NamedTempFile::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_newt-todo-mcp"))
        .env("NEWT_TODO_DB", db.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "smoke", "version": "0.0.0" }
        }
    });

    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", init).unwrap();
    stdin.flush().unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    child.kill().ok();
    child.wait().ok();

    assert!(
        line.contains("\"result\""),
        "expected a JSON-RPC result, got: {line}"
    );
    assert!(
        line.contains("serverInfo"),
        "expected serverInfo in response, got: {line}"
    );
}
