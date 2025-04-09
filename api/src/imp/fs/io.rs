use core::ffi::c_int;

use axerrno::{LinuxError, LinuxResult};
use linux_raw_sys::general::iovec;
use macro_rules_attribute::apply;

use crate::{
    fd::{File, FileLike, get_file_like},
    ptr::{UserConstPtr, UserPtr},
    syscall_instrument,
};

/// Read data from the file indicated by `fd`.
///
/// Return the read size if success.
#[apply(syscall_instrument)]
pub fn sys_read(fd: i32, buf: UserPtr<u8>, len: usize) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_slice(len)?;
    debug!(
        "sys_read <= fd: {}, buf: {:p}, len: {}",
        fd,
        buf.as_ptr(),
        buf.len()
    );
    Ok(get_file_like(fd)?.read(buf)? as _)
}

pub fn sys_readv(fd: c_int, iov: UserPtr<iovec>, iocnt: usize) -> LinuxResult<isize> {
    if !(0..=1024).contains(&iocnt) {
        return Err(LinuxError::EINVAL);
    }

    let iovs = iov.get_as_mut_slice(iocnt)?;
    let mut ret = 0;
    for iov in iovs {
        if iov.iov_len == 0 {
            continue;
        }
        let buf = UserPtr::<u8>::from(iov.iov_base as usize);
        let buf = buf.get_as_mut_slice(iov.iov_len as _)?;
        debug!(
            "sys_readv <= fd: {}, buf: {:p}, len: {}",
            fd,
            buf.as_ptr(),
            buf.len()
        );

        let read = get_file_like(fd)?.read(buf)?;
        ret += read as isize;

        if read < buf.len() {
            break;
        }
    }

    Ok(ret)
}

pub fn sys_pread64(fd: c_int, buf: UserPtr<u8>, len: usize, offset: u64) -> LinuxResult<isize> {
    let buf = buf.get_as_mut_slice(len)?;
    debug!(
        "pread64 <= fd: {}, buf: {:p}, len: {}, offset: {}",
        fd,
        buf.as_ptr(),
        buf.len(),
        offset
    );
    Ok(File::from_fd(fd)?.inner().read_at(offset, buf)? as _)
}

/// Write data to the file indicated by `fd`.
///
/// Return the written size if success.
#[apply(syscall_instrument)]
pub fn sys_write(fd: i32, buf: UserConstPtr<u8>, len: usize) -> LinuxResult<isize> {
    let buf = buf.get_as_slice(len)?;
    debug!(
        "sys_write <= fd: {}, buf: {:p}, len: {}",
        fd,
        buf.as_ptr(),
        buf.len()
    );
    Ok(get_file_like(fd)?.write(buf)? as _)
}

#[apply(syscall_instrument)]
pub fn sys_writev(fd: i32, iov: UserConstPtr<iovec>, iocnt: usize) -> LinuxResult<isize> {
    if !(0..=1024).contains(&iocnt) {
        return Err(LinuxError::EINVAL);
    }

    let iovs = iov.get_as_slice(iocnt)?;
    let mut ret = 0;
    for iov in iovs {
        if iov.iov_len == 0 {
            continue;
        }
        let buf = UserConstPtr::<u8>::from(iov.iov_base as usize);
        let buf = buf.get_as_slice(iov.iov_len as _)?;
        debug!(
            "sys_writev <= fd: {}, buf: {:p}, len: {}",
            fd,
            buf.as_ptr(),
            buf.len()
        );

        let written = get_file_like(fd)?.write(buf)?;
        ret += written as isize;

        if written < buf.len() {
            break;
        }
    }

    Ok(ret)
}
