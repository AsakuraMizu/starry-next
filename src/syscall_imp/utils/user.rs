pub(crate) fn sys_getuid() -> i32 {
    10
}

pub(crate) fn sys_getgid() -> i32 {
    10
}

pub(crate) fn sys_setuid(uid: u32) -> i32 {
    debug!("sys_setuid: {}", uid);
    0
}

pub(crate) fn sys_setgid(gid: u32) -> i32 {
    debug!("sys_setgid: {}", gid);
    0
}
