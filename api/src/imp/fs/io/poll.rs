use axerrno::LinuxResult;
use axhal::time::{TimeValue, wall_time};
use axsignal::SignalSet;
use linux_raw_sys::general::{POLLERR, POLLIN, POLLNVAL, POLLOUT, pollfd, timespec};

use crate::{
    file::get_file_like,
    ptr::{UserConstPtr, UserPtr, nullable},
    signal::check_sigset_size,
    time::TimeValueLike,
};

fn do_poll(
    fds: &mut [pollfd],
    timeout: Option<TimeValue>,
    sigmask: Option<SignalSet>,
) -> LinuxResult<isize> {
    debug!(
        "do_poll <= fds: {:?}, timeout: {:?}, sigmask: {:?}",
        fds, timeout, sigmask
    );

    // TODO: handle signals

    let deadline = timeout.map(|t| wall_time() + t);

    loop {
        axnet::poll_interfaces();

        let mut res = 0;
        for fd in &mut *fds {
            let mut revents = 0;
            match get_file_like(fd.fd) {
                Ok(f) => match f.poll() {
                    Ok(state) => {
                        if (fd.events & POLLIN as i16) != 0 && state.readable {
                            revents |= POLLIN;
                        }
                        if (fd.events & POLLOUT as i16) != 0 && state.writable {
                            revents |= POLLOUT;
                        }
                    }
                    Err(e) => {
                        warn!("poll fd={} error: {:?}", fd.fd, e);
                        revents = POLLERR;
                    }
                },
                Err(_) => {
                    revents = POLLNVAL;
                }
            }
            fd.revents = revents as _;
            if revents != 0 {
                res += 1;
            }
        }

        if res > 0 {
            return Ok(res);
        }

        if deadline.is_some_and(|d| wall_time() >= d) {
            return Ok(0);
        }

        axtask::yield_now();
    }
}

pub fn sys_poll(fds: UserPtr<pollfd>, nfds: u32, timeout: i32) -> LinuxResult<isize> {
    let fds = fds.get_as_mut_slice(nfds as usize)?;
    let timeout = if timeout < 0 {
        None
    } else {
        Some(TimeValue::from_millis(timeout as u64))
    };
    do_poll(fds, timeout, None)
}

pub fn sys_ppoll(
    fds: UserPtr<pollfd>,
    nfds: u32,
    timeout: UserConstPtr<timespec>,
    sigmask: UserConstPtr<SignalSet>,
    sigsetsize: usize,
) -> LinuxResult<isize> {
    if !sigmask.is_null() {
        check_sigset_size(sigsetsize)?;
    }

    let fds = fds.get_as_mut_slice(nfds as usize)?;
    let timeout = nullable!(timeout.get_as_ref())?.map(|ts| ts.to_time_value());
    let sigmask = nullable!(sigmask.get_as_ref())?.copied();
    do_poll(fds, timeout, sigmask)
}
