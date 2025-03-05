#![no_std]
#![no_main]
#![doc = include_str!("../README.md")]

#[macro_use]
extern crate log;
extern crate alloc;
extern crate axstd;

mod ctypes;

mod mm;
mod syscall_imp;
mod task;
use alloc::{string::ToString, sync::Arc};

use axhal::arch::UspaceContext;
use axstd::println;
use axsync::Mutex;
use memory_addr::VirtAddr;

fn run_user_app<'a>(program: &'a str, args: impl IntoIterator<Item = &'a str>) -> Option<i32> {
    let mut args = core::iter::once(program)
        .chain(args)
        .map(|s| s.to_string())
        .collect();

    let mut uspace = axmm::new_user_aspace(
        VirtAddr::from_usize(axconfig::plat::USER_SPACE_BASE),
        axconfig::plat::USER_SPACE_SIZE,
    )
    .expect("Failed to create user address space");

    let (entry_vaddr, ustack_top) = mm::load_user_app(&mut args, &mut uspace)
        .unwrap_or_else(|e| panic!("Failed to load user app {}: {}", program, e));
    let user_task = task::spawn_user_task(
        Arc::new(Mutex::new(uspace)),
        UspaceContext::new(entry_vaddr.into(), ustack_top, 2333),
        0,
    );
    user_task.join()
}

const BASIC_TESTCASES: &[&str] = &[
    "brk",
    "chdir",
    "clone",
    "close",
    "dup",
    "dup2",
    "execve",
    "exit",
    "fork",
    "fstat",
    "getcwd",
    "getdents",
    "getpid",
    "getppid",
    "gettimeofday",
    "mkdir_",
    "mmap",
    "mount",
    "munmap",
    "open",
    "openat",
    "pipe",
    "read",
    "sleep",
    "times",
    "umount",
    "uname",
    "unlink",
    "wait",
    "waitpid",
    "write",
    "yield",
];

#[allow(unused)]
fn basic_test() {
    println!("#### OS COMP TEST GROUP START basic-musl ####");
    axfs::api::set_current_dir("/musl/basic").expect("Failed to set current dir");
    for testcase in BASIC_TESTCASES {
        println!("Testing {}:", testcase);
        let exit_code = run_user_app(testcase, []);
        info!("User task {} exited with code: {:?}", testcase, exit_code);
    }
    println!("#### OS COMP TEST GROUP END basic-musl ####");
}

fn busybox_test() {
    axfs::api::set_current_dir("/musl").expect("Failed to set current dir");
    let exit_code = run_user_app("busybox", ["echo", "1"]);
    info!("busybox exited with code: {:?}", exit_code);
}

#[unsafe(no_mangle)]
fn main() {
    busybox_test();
}
