use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use tempfile::tempdir;

#[test]
#[ignore = "starts the desktop UI; run explicitly in an environment with display access"]
fn smoke_test_mode_starts_and_exits() {
    let home = tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_skills-manager-ui"))
        .arg("--smoke-test")
        .env("HOME", home.path())
        .env("RUST_LOG", "off")
        .spawn()
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "UI smoke test exited with {status}");
            return;
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("UI smoke test did not exit within 5 seconds");
        }

        thread::sleep(Duration::from_millis(50));
    }
}
