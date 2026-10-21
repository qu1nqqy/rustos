use crate::{print, println};
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::stream::{Stream, StreamExt};
use futures_util::task::AtomicWaker;
use pc_keyboard::{DecodedKey, HandleControl, PS2Keyboard, ScancodeSet1, layouts};

// Очередь создаётся в ScancodeStream::new, а не в static напрямую: ArrayQueue
// выделяет память в куче, а в обработчике прерывания этого делать нельзя
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Вызывается обработчиком прерывания клавиатуры.
///
/// Не должна блокироваться и выделять память
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
}

/// Асинхронный поток скан-кодов клавиатуры
pub struct ScancodeStream {
    // приватное поле не даёт создать структуру в обход new
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // быстрый путь: скан-код уже в очереди
        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        // waker регистрируется до второй проверки: если прерывание придёт
        // между ними, скан-код не потеряется
        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

/// Задача, которая печатает нажатые клавиши
pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = PS2Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
