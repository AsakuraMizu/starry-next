use axsignal::ctypes::SignalInfo;
use axtask::{TaskExtRef, current};
use linux_raw_sys::general::{SI_KERNEL, SIGKILL};

use crate::{fd::FD_TABLE, send_signal_thread};

pub fn do_exit(exit_code: i32, group_exit: bool) -> ! {
    let curr = current();
    let clear_child_tid = curr.task_ext().thread_data().clear_child_tid() as *mut i32;
    if !clear_child_tid.is_null() {
        // TODO: check whether the address is valid
        unsafe {
            // TODO: Encapsulate all operations that access user-mode memory into a unified function
            *(clear_child_tid) = 0;
        }
        // TODO: wake up threads, which are blocked by futex, and waiting for the address pointed by clear_child_tid
    }

    let thread = &curr.task_ext().thread;
    let process = thread.process();
    if thread.exit(exit_code) {
        // TODO: send exit signal to parent
        process.exit();
        // TODO: clear namespace resources
        // FIXME: axns should drop all the resources
        FD_TABLE.clear();
    }
    if group_exit {
        process.group_exit();
        let sig = SignalInfo::new(SIGKILL, SI_KERNEL);
        for thr in process.threads() {
            send_signal_thread(&*thr, sig.clone());
        }
    }
    axtask::exit(exit_code)
}

pub fn sys_exit(exit_code: i32) -> ! {
    do_exit(exit_code << 8, false)
}

pub fn sys_exit_group(exit_code: i32) -> ! {
    do_exit(exit_code << 8, true)
}
