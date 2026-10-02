#[test_case]
fn test_serial_println() {
    serial_println!();
    serial_println!("test_serial_println output");
    serial_println!("{} + {} = {}", 2, 2, 4);
}
