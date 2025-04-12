mod clone;
mod execve;
mod exit;
mod schedule;
mod thread;
mod wait;

pub use self::clone::*;
pub use self::execve::*;
pub use self::exit::*;
pub use self::schedule::*;
pub use self::thread::*;
pub use self::wait::*;

pub fn on_task_enter() {
    use crate::ptr::{UserPtr, nullable};
    use axtask::{TaskExtRef, current};

    let curr = current();
    let set_child_tid: UserPtr<u32> = curr
        .task_ext()
        .thread_data()
        .set_child_tid
        .load(core::sync::atomic::Ordering::Relaxed)
        .into();
    if let Ok(Some(tid)) = nullable!(set_child_tid.get_as_mut()) {
        *tid = curr.task_ext().thread.tid();
    }
}
