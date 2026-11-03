use super::*;
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::{RawWaker, RawWakerVTable, Waker};

// stream() можно вызвать у очереди только один раз, поэтому у каждого теста
// своя static-очередь

static WOKEN: AtomicBool = AtomicBool::new(false);

/// Waker, который только запоминает, что его разбудили
fn flag_waker() -> Waker {
    fn clone(_: *const ()) -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }
    fn wake(_: *const ()) {
        WOKEN.store(true, Ordering::SeqCst);
    }
    fn drop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake, drop);

    unsafe { Waker::from_raw(clone(core::ptr::null())) }
}

fn poll<T>(stream: &mut QueueStream<T>) -> Poll<Option<T>> {
    let waker = flag_waker();
    let mut context = Context::from_waker(&waker);
    Pin::new(stream).poll_next(&mut context)
}

#[test_case]
fn test_push_without_reader_fails() {
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    assert_eq!(QUEUE.push(1), Err(PushError::Uninit));
}

#[test_case]
fn test_push_after_stream_succeeds() {
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let _stream = QUEUE.stream(4);
    assert_eq!(QUEUE.push(1), Ok(()));
}

#[test_case]
fn test_push_to_full_queue_fails() {
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let _stream = QUEUE.stream(2);
    assert_eq!(QUEUE.push(1), Ok(()));
    assert_eq!(QUEUE.push(2), Ok(()));
    assert_eq!(QUEUE.push(3), Err(PushError::Full));
}

#[test_case]
fn test_stream_returns_items_in_order() {
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let mut stream = QUEUE.stream(4);
    QUEUE.push(1).unwrap();
    QUEUE.push(2).unwrap();
    QUEUE.push(3).unwrap();
    assert_eq!(poll(&mut stream), Poll::Ready(Some(1)));
    assert_eq!(poll(&mut stream), Poll::Ready(Some(2)));
    assert_eq!(poll(&mut stream), Poll::Ready(Some(3)));
}

#[test_case]
fn test_empty_stream_is_pending() {
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let mut stream = QUEUE.stream(4);
    assert_eq!(poll(&mut stream), Poll::Pending);
}

#[test_case]
fn test_stream_never_ends() {
    // поток бесконечный: пустая очередь - это Pending, а не конец (None)
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let mut stream = QUEUE.stream(4);
    QUEUE.push(7).unwrap();
    assert_eq!(poll(&mut stream), Poll::Ready(Some(7)));
    assert_eq!(poll(&mut stream), Poll::Pending);
    QUEUE.push(8).unwrap();
    assert_eq!(poll(&mut stream), Poll::Ready(Some(8)));
}

#[test_case]
fn test_push_wakes_pending_reader() {
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let mut stream = QUEUE.stream(4);
    assert_eq!(poll(&mut stream), Poll::Pending);
    WOKEN.store(false, Ordering::SeqCst);
    QUEUE.push(1).unwrap();
    assert!(WOKEN.load(Ordering::SeqCst));
}

#[test_case]
fn test_push_without_pending_reader_does_not_wake() {
    // waker регистрируется только когда очередь пуста
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let _stream = QUEUE.stream(4);
    WOKEN.store(false, Ordering::SeqCst);
    QUEUE.push(1).unwrap();
    assert!(!WOKEN.load(Ordering::SeqCst));
}

#[test_case]
fn test_failed_push_does_not_wake() {
    static QUEUE: AsyncQueue<u8> = AsyncQueue::new();
    let mut stream = QUEUE.stream(1);
    QUEUE.push(1).unwrap();
    assert_eq!(poll(&mut stream), Poll::Ready(Some(1)));
    assert_eq!(poll(&mut stream), Poll::Pending);
    QUEUE.push(2).unwrap();
    WOKEN.store(false, Ordering::SeqCst);
    assert_eq!(QUEUE.push(3), Err(PushError::Full));
    assert!(!WOKEN.load(Ordering::SeqCst));
}

#[test_case]
fn test_queue_works_with_other_types() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Event(u32);

    static QUEUE: AsyncQueue<Event> = AsyncQueue::new();
    let mut stream = QUEUE.stream(4);
    QUEUE.push(Event(42)).unwrap();
    assert_eq!(poll(&mut stream), Poll::Ready(Some(Event(42))));
}
