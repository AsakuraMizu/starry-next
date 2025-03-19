use core::ffi::{c_int, c_short, c_ulong, c_void};

use arceos_posix_api::{self as api, ctypes::timespec};
use axerrno::LinuxResult;
use axhal::time::{TimeValue, wall_time};
use macro_rules_attribute::apply;

use crate::{
    ptr::{PtrWrapper, UserConstPtr, UserPtr},
    syscall_imp::syscall_instrument,
};

#[repr(C)]
pub struct PollFd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

bitflags::bitflags! {
    struct PollEvents: c_short {
        /// There is data to read.
        const IN = 0x001;
        /// There is urgent data to read.
        const PRI = 0x002;
        /// Writing now will not block.
        const OUT = 0x004;
        /// Error condition.
        const ERR = 0x008;
        /// Hang up.
        const HUP = 0x010;
        /// Invalid polling request.
        const NVAL = 0x020;
    }
}

fn poll_impl(fds: &mut [PollFd], timeout: Option<TimeValue>) -> LinuxResult<isize> {
    let deadline = timeout.map(|timeout| wall_time() + timeout);

    loop {
        axnet::poll_interfaces();

        let mut res = 0;
        for pollfd in fds.iter_mut() {
            let events = PollEvents::from_bits_truncate(pollfd.events);
            let mut revents = PollEvents::empty();
            match api::get_file_like(pollfd.fd) {
                Ok(fd) => match fd.poll() {
                    Ok(state) => {
                        if state.readable && events.contains(PollEvents::IN) {
                            revents |= PollEvents::IN;
                        }
                        if state.writable && events.contains(PollEvents::OUT) {
                            revents |= PollEvents::OUT;
                        }
                    }
                    Err(e) => {
                        debug!("    except: {} {:?}", pollfd.fd, e);
                        revents |= PollEvents::ERR;
                    }
                },
                Err(_) => {
                    revents |= PollEvents::NVAL;
                }
            }
            pollfd.revents = revents.bits();
            if !revents.is_empty() {
                res += 1;
            }
        }

        if res > 0 {
            return Ok(res);
        }

        if deadline.is_some_and(|ddl| wall_time() >= ddl) {
            debug!("    timeout!");
            return Ok(0);
        }
        api::sys_sched_yield();
    }
}

#[apply(syscall_instrument)]
pub fn sys_poll(fds: UserPtr<PollFd>, nfds: c_ulong, timeout: c_int) -> LinuxResult<isize> {
    let fds = unsafe { core::slice::from_raw_parts_mut(fds.get()?, nfds as _) };
    poll_impl(
        fds,
        if timeout >= 0 {
            Some(TimeValue::from_millis(timeout as _))
        } else {
            None
        },
    )
}

#[apply(syscall_instrument)]
pub fn sys_ppoll(
    fds: UserPtr<PollFd>,
    nfds: c_ulong,
    timeout: UserConstPtr<timespec>,
    _sigmask: UserConstPtr<c_void>,
) -> LinuxResult<isize> {
    let fds = unsafe { core::slice::from_raw_parts_mut(fds.get()?, nfds as _) };
    poll_impl(
        fds,
        timeout
            .nullable(UserConstPtr::get)?
            .map(|t| unsafe { (*t).into() }),
    )
}
