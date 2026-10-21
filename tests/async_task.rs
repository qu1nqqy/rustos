#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(rustos::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

mod common;

use alloc::{vec, vec::Vec};
use bootloader::{BootInfo, entry_point};
use common::yield_now;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, Ordering};
use rustos::task::{Task, simple_executor::SimpleExecutor};
use spin::Mutex;

// Задачи хранятся в куче, поэтому нужен init_heap и отдельный образ.
// Здесь проверяется SimpleExecutor: его run возвращает управление, когда
// задачи кончились. Executor::run бесконечный, он проверяется в tests/executor.rs

entry_point!(test_kernel_main);

fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    common::init_heap(boot_info);

    test_main();
    rustos::hlt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rustos::test_panic_handler(info)
}

// порядок, в котором задачи выполняли свои шаги
static LOG: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn run_tasks(tasks: Vec<Task>) -> Vec<u32> {
    LOG.lock().clear();
    let mut executor = SimpleExecutor::new();
    for task in tasks {
        executor.spawn(task);
    }
    executor.run();
    core::mem::take(&mut *LOG.lock())
}

fn log(step: u32) {
    LOG.lock().push(step);
}

async fn async_number() -> u32 {
    42
}

#[test_case]
fn test_run_without_tasks_returns() {
    SimpleExecutor::new().run();
}

#[test_case]
fn test_task_runs_to_completion() {
    static DONE: AtomicBool = AtomicBool::new(false);
    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(async {
        DONE.store(true, Ordering::SeqCst);
    }));
    executor.run();
    assert!(DONE.load(Ordering::SeqCst));
}

#[test_case]
fn test_await_returns_value() {
    let steps = run_tasks(vec![Task::new(async {
        log(async_number().await);
    })]);
    assert_eq!(steps, [42]);
}

#[test_case]
fn test_tasks_run_in_spawn_order() {
    let steps = run_tasks(vec![
        Task::new(async { log(1) }),
        Task::new(async { log(2) }),
        Task::new(async { log(3) }),
    ]);
    assert_eq!(steps, [1, 2, 3]);
}

#[test_case]
fn test_pending_task_polled_again() {
    // первая задача уступает очередь на yield_now и продолжается после второй
    let steps = run_tasks(vec![
        Task::new(async {
            log(1);
            yield_now().await;
            log(3);
        }),
        Task::new(async { log(2) }),
    ]);
    assert_eq!(steps, [1, 2, 3]);
}

#[test_case]
fn test_tasks_interleave() {
    async fn worker(id: u32) {
        for step in 0..3 {
            log(id * 10 + step);
            yield_now().await;
        }
    }
    let steps = run_tasks(vec![Task::new(worker(1)), Task::new(worker(2))]);
    assert_eq!(steps, [10, 20, 11, 21, 12, 22]);
}

#[test_case]
fn test_task_owns_moved_data() {
    // async move забирает вектор внутрь задачи, он живёт столько же, сколько задача
    let data = vec![1, 2, 3];
    let steps = run_tasks(vec![Task::new(async move {
        yield_now().await;
        log(data.iter().sum());
    })]);
    assert_eq!(steps, [6]);
}
