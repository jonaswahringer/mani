use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

const TIMEOUT: Duration = Duration::from_secs(8);

fn write_executable(path: &std::path::Path, source: &str) {
    fs::write(path, source).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn receive_until(receiver: &Receiver<Vec<u8>>, output: &mut Vec<u8>, needle: &[u8]) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(chunk) => {
                output.extend(chunk);
                if output.windows(needle.len()).any(|window| window == needle) {
                    return true;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    false
}

#[test]
fn interactive_view_opens_in_a_pty_and_restores_the_terminal_on_quit() {
    let home = tempfile::tempdir().unwrap();
    let bin = tempfile::tempdir().unwrap();
    fs::write(
        home.path().join("agent-topic.md"),
        "# Agent Topic\n\nSearchable introduction.\n\n## Details\n\nMore help.\n",
    )
    .unwrap();
    write_executable(
        &bin.path().join("man"),
        "#!/bin/sh\nif [ \"$1\" = '-w' ]; then\n  printf '/usr/share/man/man1/agent-topic.1.gz\\n'\nelse\n  printf 'AGENT-TOPIC(1)\\n\\nSearchable official text.\\n'\nfi\n",
    );

    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_mani"));
    command.arg("agent-topic");
    command.env("MANI_HOME", home.path());
    command.env(
        "PATH",
        format!(
            "{}:{}",
            bin.path().display(),
            std::env::var("PATH").unwrap_or_default()
        ),
    );
    command.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(command).unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader_thread = thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    if sender.send(buffer[..read].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut output = Vec::new();
    if !receive_until(&receiver, &mut output, b"CUSTOM") {
        let _ = child.kill();
        panic!("Interactive View did not render before timeout");
    }
    writer.write_all(b"/Searchable\r").unwrap();
    writer.flush().unwrap();
    if !receive_until(&receiver, &mut output, b"1 of 1 matches") {
        let _ = child.kill();
        panic!("Interactive View search did not complete before timeout");
    }
    writer.write_all(b"\t").unwrap();
    writer.flush().unwrap();
    if !receive_until(&receiver, &mut output, b"OFFICIAL") {
        let _ = child.kill();
        panic!("Interactive View did not switch sources before timeout");
    }
    writer.write_all(b"q").unwrap();
    writer.flush().unwrap();

    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("Interactive View did not quit before timeout");
        }
        thread::sleep(Duration::from_millis(20));
    };
    drop(writer);
    drop(pair.master);
    reader_thread.join().unwrap();
    while let Ok(chunk) = receiver.try_recv() {
        output.extend(chunk);
    }

    assert!(status.success());
    assert!(
        output
            .windows(b"CUSTOM".len())
            .any(|window| window == b"CUSTOM")
    );
    assert!(
        output
            .windows(b"OFFICIAL".len())
            .any(|window| window == b"OFFICIAL")
    );
    assert!(
        output
            .windows(b"1 of 1 matches".len())
            .any(|window| window == b"1 of 1 matches")
    );
    assert!(
        output
            .windows(b"\x1b[?1049h".len())
            .any(|window| window == b"\x1b[?1049h")
    );
    assert!(
        output
            .windows(b"\x1b[?1049l".len())
            .any(|window| window == b"\x1b[?1049l")
    );
    assert!(
        output
            .windows(b"\x1b[?25h".len())
            .any(|window| window == b"\x1b[?25h")
    );
}
