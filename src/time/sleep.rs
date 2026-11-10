use super::{duration_to_ticks, ticks};
use crate::arch::x86_64::without_interrupts;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use core::time::Duration;
use spin::Mutex;

pub const MAX_SLEEPERS: usize = 32;

struct Sleeper {
    deadline: u64,
    waker: Waker,
    woken: bool,
}

const EMPTY: Option<Sleeper> = None;
static SLEEPERS: Mutex<[Option<Sleeper>; MAX_SLEEPERS]> = Mutex::new([EMPTY; MAX_SLEEPERS]);

pub fn sleep(duration: Duration) -> Sleep {
    Sleep {
        deadline: ticks() + duration_to_ticks(duration),
        slot: None,
    }
}

pub struct Sleep {
    deadline: u64,
    slot: Option<usize>,
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<()> {
        // прерывания выключены, пока мы держим SLEEPERS: это lock берёт
        // и обработчик таймера
        without_interrupts(|| {
            let mut sleepers = SLEEPERS.lock();
            // время проверяется под lock'ом: иначе тик между проверкой и
            // записью разбудил бы всех, кроме нас, и мы проспали бы вечно
            if ticks() >= self.deadline {
                if let Some(slot) = self.slot.take() {
                    sleepers[slot] = None;
                }
                return Poll::Ready(());
            }
            let slot = match self.slot {
                Some(slot) => slot,
                None => {
                    let free = sleepers.iter().position(Option::is_none);
                    let slot = free.expect("too many sleeping tasks");
                    self.slot = Some(slot);
                    slot
                }
            };
            sleepers[slot] = Some(Sleeper {
                deadline: self.deadline,
                waker: cx.waker().clone(),
                woken: false,
            });
            Poll::Pending
        })
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if let Some(slot) = self.slot {
            without_interrupts(|| SLEEPERS.lock()[slot] = None)
        }
    }
}

pub(super) fn wake_expired(now: u64) {
    for sleeper in SLEEPERS.lock().iter_mut().flatten() {
        if sleeper.deadline <= now && !sleeper.woken {
            sleeper.woken = true;
            sleeper.waker.wake_by_ref();
        }
    }
}

pub fn sleeping() -> usize {
    without_interrupts(|| SLEEPERS.lock().iter().flatten().count())
}

#[cfg(test)]
mod tests;
