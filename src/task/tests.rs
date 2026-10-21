use super::*;

// Task::new кладёт future в кучу, а в unit-тестах куча не инициализирована,
// поэтому здесь только TaskId. Задачи и executor'ы проверяются в
// tests/async_task.rs и tests/executor.rs

#[test_case]
fn test_task_ids_unique() {
    let first = TaskId::new();
    let second = TaskId::new();
    let third = TaskId::new();
    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_ne!(first, third);
}

#[test_case]
fn test_task_ids_increasing() {
    // BTreeMap в Executor хранит задачи упорядоченными по id
    let first = TaskId::new();
    let second = TaskId::new();
    assert!(first < second);
    assert_eq!(second.0, first.0 + 1);
}
