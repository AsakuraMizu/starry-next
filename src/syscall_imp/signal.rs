use core::ffi::c_void;

pub(crate) fn sys_rt_sigprocmask(
    _how: i32,
    _set: *const c_void,
    _oldset: *mut c_void,
    _sigsetsize: usize,
) -> isize {
    0
}
