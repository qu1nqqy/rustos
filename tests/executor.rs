#![no_std]
#![no_main]

extern crate alloc;

mod common;

use bootloader::{BootInfo, entry_point};
use common::yield_now;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicUsize, Ordering};
use futures_util::stream::StreamExt;
use rustos::drivers::keyboard::ScancodeStream;
use rustos::task::{Task, executor::Executor};
use rustos::{QemuExitCode, exit_qemu, serial_print, serial_println};
use x86_64::instructions::port::Port;

// Executor::run никогда не возвращается, поэтому тест без раннера: последняя
// задача сама завершает QEMU. Если executor не разбудит задачу, тест зависнет
// и упадёт по test-timeout.
//
// Задачи:
// - workers: по несколько раз уступают очередь (waker сразу будит задачу);
// - keyboard: ждёт скан-код из ScancodeStream. Её будит обработчик прерывания
//   клавиатуры, а до этого executor спит в hlt, потому что очередь пуста;
// - presser: эмулирует нажатие клавиши (как в tests/keyboard.rs), но не ждёт
//   обработчика, поэтому прерывание приходит, когда executor уже уснул

const WORKERS: usize = 3;
const A_PRESSED: u8 = 0x1e;

static WORKERS_DONE: AtomicUsize = AtomicUsize::new(0);

entry_point!(test_kernel_main);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    serial_print!("executor::tasks_and_keyboard_wakeup...\t");

    rustos::init();
    common::init_heap(boot_info);

    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard_task()));
    for i in 0..WORKERS {
        executor.spawn(Task::new(worker(i + 1)));
    }
    executor.spawn(Task::new(presser()));
    executor.run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}

async fn worker(yields: usize) {
    for _ in 0..yields {
        yield_now().await;
    }
    WORKERS_DONE.fetch_add(1, Ordering::SeqCst);
}

async fn presser() {
    // keyboard_task уже создала очередь и ждёт скан-код
    let status = || unsafe { Port::<u8>::new(0x64).read() };
    while status() & 0b10 != 0 {}
    unsafe { Port::<u8>::new(0x64).write(0xd2u8) };
    while status() & 0b10 != 0 {}
    unsafe { Port::<u8>::new(0x60).write(A_PRESSED) };
}

async fn keyboard_task() {
    let mut scancodes = ScancodeStream::new();
    let scancode = scancodes.next().await;
    assert_eq!(scancode, Some(A_PRESSED));

    // workers могли ещё не закончить: уступаем им очередь
    while WORKERS_DONE.load(Ordering::SeqCst) < WORKERS {
        yield_now().await;
    }

    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
}
