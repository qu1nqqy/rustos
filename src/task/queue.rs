use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;

/// Очередь, которую наполняет обработчик прерывания, а читает async-задача.
///
/// Объявляется как `static`. Сама очередь создаётся в куче при первом вызове
/// `stream`: обработчику прерывания выделять память нельзя
pub struct AsyncQueue<T> {
    queue: OnceCell<ArrayQueue<T>>,
    waker: AtomicWaker,
}

/// Почему `push` не положил элемент в очередь
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushError {
    /// Читателя ещё нет: `stream` ни разу не вызывали
    Uninit,
    /// Читатель не успевает разбирать очередь
    Full,
}

impl<T> AsyncQueue<T> {
    pub const fn new() -> Self {
        AsyncQueue {
            queue: OnceCell::uninit(),
            waker: AtomicWaker::new(),
        }
    }

    /// Кладёт элемент в очередь и будит читателя.
    ///
    /// Не блокируется и не выделяет память, поэтому подходит для обработчиков прерываний
    pub fn push(&self, item: T) -> Result<(), PushError> {
        let queue = self.queue.try_get().map_err(|_| PushError::Uninit)?;
        queue.push(item).map_err(|_| PushError::Full)?;
        self.waker.wake();
        Ok(())
    }

    /// Создаёт очередь на `capacity` элементов и возвращает поток для чтения из неё.
    ///
    /// Читатель у очереди один, поэтому вызвать можно только один раз
    pub fn stream(&'static self, capacity: usize) -> QueueStream<T> {
        self.queue
            .try_init_once(|| ArrayQueue::new(capacity))
            .expect("AsyncQueue::stream should only be called once");
        QueueStream { queue: self }
    }
}

/// Асинхронный поток элементов из `AsyncQueue`
pub struct QueueStream<T: 'static> {
    queue: &'static AsyncQueue<T>,
}

impl<T> Stream for QueueStream<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<T>> {
        let queue = self.queue.queue.try_get().expect("queue not initialized");

        // быстрый путь: элемент уже в очереди
        if let Some(item) = queue.pop() {
            return Poll::Ready(Some(item));
        }

        // waker регистрируется до второй проверки: если прерывание придёт
        // между ними, элемент не потеряется
        self.queue.waker.register(cx.waker());
        match queue.pop() {
            Some(item) => {
                self.queue.waker.take();
                Poll::Ready(Some(item))
            }
            None => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests;
