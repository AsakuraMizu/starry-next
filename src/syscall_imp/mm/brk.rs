use axhal::paging::MappingFlags;
use axtask::{TaskExtRef, current};
use memory_addr::{MemoryAddr, VirtAddr};

use crate::syscall_body;

const MAX_HEAP_SIZE: usize = 0x20000;

pub fn sys_brk(addr: usize) -> isize {
    debug!("sys_brk: addr: {:#x}", addr);
    syscall_body!(sys_brk, {
        let current_task = current();

        let heap_bottom = current_task.task_ext().get_heap_bottom();
        if addr != 0 && addr >= heap_bottom && addr <= heap_bottom + MAX_HEAP_SIZE {
            let old_top =
                VirtAddr::from_usize(current_task.task_ext().get_heap_top()).align_up_4k();
            let new_top = VirtAddr::from_usize(addr).align_up_4k();

            match new_top.cmp(&old_top) {
                core::cmp::Ordering::Less => {
                    let size = old_top - new_top;
                    let mut aspace = current_task.task_ext().aspace.lock();
                    aspace.unmap(new_top, size)?;
                }
                core::cmp::Ordering::Greater => {
                    let size = new_top - old_top;
                    let mut aspace = current_task.task_ext().aspace.lock();
                    aspace.map_alloc(
                        old_top,
                        size,
                        MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
                        false,
                    )?;
                }
                core::cmp::Ordering::Equal => {}
            }

            current_task.task_ext().set_heap_top(addr);
            Ok(addr as isize)
        } else {
            Ok(current_task.task_ext().get_heap_top() as isize)
        }
    })
}
