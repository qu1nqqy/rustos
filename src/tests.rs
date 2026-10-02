use super::*;

#[test_case]
fn test_exit_code_matches_bootimage_config() {
    // QEMU завершается с кодом (value << 1) | 1, а bootimage считает
    // успехом test-success-exit-code = 33 из Cargo.toml
    assert_eq!((QemuExitCode::Success as u32) << 1 | 1, 33);
    assert_ne!(QemuExitCode::Success, QemuExitCode::Failed);
}
