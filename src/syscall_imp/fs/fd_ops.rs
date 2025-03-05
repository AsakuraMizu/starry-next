use core::ffi::{c_char, c_int};

use arceos_posix_api::{self as api, ctypes::mode_t};

pub(crate) fn sys_dup(old_fd: c_int) -> c_int {
    api::sys_dup(old_fd)
}

pub(crate) fn sys_dup3(old_fd: c_int, new_fd: c_int) -> c_int {
    api::sys_dup2(old_fd, new_fd)
}

pub(crate) fn sys_openat(dirfd: i32, path: *const c_char, flags: i32, modes: mode_t) -> isize {
    api::sys_openat(dirfd, path, flags, modes) as isize
}

pub(crate) fn sys_close(fd: c_int) -> c_int {
    api::sys_close(fd)
}
