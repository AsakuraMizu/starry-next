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
use alloc::{string::ToString, sync::Arc, vec};

use axhal::arch::UspaceContext;
use axstd::println;
use axsync::Mutex;
use memory_addr::VirtAddr;

#[allow(unused)]
const BASIC_TESTCASES_PASSED: &[&str] = &[
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
    "sleep",
    "getcwd",
    "getdents",
    "getpid",
    "getppid",
    "gettimeofday",
    "mkdir_",
    "mmap",
    "munmap",
    "open",
    "openat",
    "pipe",
    "read",
    "times",
    "uname",
    "unlink",
    "wait",
    "waitpid",
    "write",
    "yield",
];

const BASIC_TESTCASES: &[&str] = &[
    "execve", // failed on loongarch64
    // not implemented
    "mount", "umount",
];

#[unsafe(no_mangle)]
fn main() {
    println!("#### OS COMP TEST GROUP START basic-musl ####");
    axfs::api::set_current_dir("/musl/basic").expect("Failed to set current dir");
    for testcase in BASIC_TESTCASES {
        println!("Testing {}:", testcase);

        let args = vec![testcase.to_string()];
        let mut uspace = axmm::new_user_aspace(
            VirtAddr::from_usize(axconfig::plat::USER_SPACE_BASE),
            axconfig::plat::USER_SPACE_SIZE,
        )
        .expect("Failed to create user address space");
        let (entry_vaddr, ustack_top) = mm::load_user_app(&mut (args.into()), &mut uspace)
            .unwrap_or_else(|e| panic!("Failed to load user app {}: {}", testcase, e));
        let user_task = task::spawn_user_task(
            Arc::new(Mutex::new(uspace)),
            UspaceContext::new(entry_vaddr.into(), ustack_top, 2333),
            0,
        );
        let exit_code = user_task.join();
        info!("User task {} exited with code: {:?}", testcase, exit_code);
    }
    println!("#### OS COMP TEST GROUP END basic-musl ####");
}
