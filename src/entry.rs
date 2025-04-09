use alloc::{string::String, sync::Arc};
use axfs::{CURRENT_DIR, CURRENT_DIR_PATH, api::set_current_dir};
use axhal::{
    arch::UspaceContext,
    mem::{PAGE_SIZE_4K, virt_to_phys},
    paging::MappingFlags,
};
use axprocess::{Pid, ProcessBuilder, ThreadBuilder};
use axsync::Mutex;
use starry_api::fd::FD_TABLE;
use starry_core::{
    mm::{copy_from_kernel, load_user_app, new_user_aspace_empty},
    task::{ProcessData, TaskExt, ThreadData, add_thread_to_table, new_user_task},
};

unsafe extern "C" {
    fn start_signal_trampoline();
}

pub fn run_user_app(args: &[String], envs: &[String]) -> Option<i32> {
    let mut uspace = new_user_aspace_empty()
        .and_then(|mut it| {
            copy_from_kernel(&mut it)?;
            let signal_trampoline_paddr = virt_to_phys((start_signal_trampoline as usize).into());
            it.map_linear(
                axconfig::plat::SIGNAL_TRAMPOLINE.into(),
                signal_trampoline_paddr,
                PAGE_SIZE_4K,
                MappingFlags::READ | MappingFlags::EXECUTE | MappingFlags::USER,
            )?;
            Ok(it)
        })
        .expect("Failed to create user address space");

    let exe_path = args[0].clone();
    let (dir, name) = exe_path.rsplit_once('/').unwrap_or(("", &exe_path));
    set_current_dir(dir).expect("Failed to set current dir");

    let (entry_vaddr, ustack_top) = load_user_app(&mut uspace, args, envs)
        .unwrap_or_else(|e| panic!("Failed to load user app: {}", e));

    let uctx = UspaceContext::new(entry_vaddr.into(), ustack_top, 2333);

    let mut task = new_user_task(name);
    task.ctx_mut().set_page_table_root(uspace.page_table_root());

    let process_data = ProcessData::new(exe_path, Arc::new(Mutex::new(uspace)));

    FD_TABLE
        .deref_from(&process_data.ns)
        .init_new(FD_TABLE.copy_inner());
    CURRENT_DIR
        .deref_from(&process_data.ns)
        .init_new(CURRENT_DIR.copy_inner());
    CURRENT_DIR_PATH
        .deref_from(&process_data.ns)
        .init_new(CURRENT_DIR_PATH.copy_inner());

    let tid = task.id().as_u64() as Pid;
    let process = ProcessBuilder::new(tid).data(process_data).build();

    let thread = ThreadBuilder::new(tid, process)
        .data(ThreadData::new())
        .build();
    add_thread_to_table(&thread);

    task.init_task_ext(TaskExt::new(uctx, thread));

    let task = axtask::spawn_task(task);
    task.join()
}
