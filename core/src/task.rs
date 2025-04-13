use core::{
    alloc::Layout,
    array,
    cell::{Cell, RefCell},
    hint::black_box,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::{
    collections::btree_map::BTreeMap,
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};
use axerrno::{LinuxError, LinuxResult};
use axhal::{
    arch::UspaceContext,
    time::{NANOS_PER_MICROS, NANOS_PER_SEC, monotonic_time_nanos},
};
use axmm::{AddrSpace, kernel_aspace};
use axns::{AxNamespace, AxNamespaceIf};
use axprocess::{Pid, Process, ProcessGroup, Session, Thread};
use axsignal::{
    PendingSignals,
    ctypes::{SignalAction, SignalSet, SignalStack},
};
use axsync::{Mutex, spin::SpinNoIrq};
use axtask::{TaskExtRef, TaskInner, WaitQueue, current};
use memory_addr::VirtAddrRange;
use spin::{Once, rwlock::RwLock};
use weak_map::WeakMap;

use crate::{resources::Rlimits, time::TimeStat};

pub fn new_user_task(name: &str, on_enter: Option<fn()>) -> TaskInner {
    TaskInner::new(
        move || {
            let curr = axtask::current();
            let kstack_top = curr.kernel_stack_top().unwrap();
            let uctx = curr.task_ext().uctx.take().unwrap();
            black_box(&uctx);
            info!(
                "Enter user space: entry={:#x}, ustack={:#x}, kstack={:#x}",
                uctx.ip(),
                uctx.sp(),
                kstack_top,
            );

            if let Some(on_enter) = on_enter {
                on_enter();
            }
            unsafe { uctx.enter_uspace(kstack_top) }
        },
        name.into(),
        axconfig::plat::KERNEL_STACK_SIZE,
    )
}

/// Task extended data for the monolithic kernel.
pub struct TaskExt {
    /// The user space context.
    pub uctx: Cell<Option<UspaceContext>>,
    /// The time statistics
    pub time: RefCell<TimeStat>,
    /// The thread
    pub thread: Arc<Thread>,
}

impl TaskExt {
    pub fn new(uctx: UspaceContext, thread: Arc<Thread>) -> Self {
        Self {
            uctx: Cell::new(Some(uctx)),
            time: RefCell::new(TimeStat::new()),
            thread,
        }
    }

    pub(crate) fn time_stat_from_kernel_to_user(&self, current_tick: usize) {
        self.time.borrow_mut().switch_into_user_mode(current_tick);
    }

    pub(crate) fn time_stat_from_user_to_kernel(&self, current_tick: usize) {
        self.time.borrow_mut().switch_into_kernel_mode(current_tick);
    }

    pub(crate) fn time_stat_output(&self) -> (usize, usize) {
        self.time.borrow().output()
    }

    pub fn thread_data(&self) -> &ThreadData {
        self.thread.data().unwrap()
    }

    pub fn process_data(&self) -> &ProcessData {
        self.thread.process().data().unwrap()
    }
}

axtask::def_task_ext!(TaskExt);

pub fn time_stat_from_kernel_to_user() {
    let curr_task = current();
    curr_task
        .task_ext()
        .time_stat_from_kernel_to_user(monotonic_time_nanos() as usize);
}

pub fn time_stat_from_user_to_kernel() {
    let curr_task = current();
    curr_task
        .task_ext()
        .time_stat_from_user_to_kernel(monotonic_time_nanos() as usize);
}

pub fn time_stat_output() -> (usize, usize, usize, usize) {
    let curr_task = current();
    let (utime_ns, stime_ns) = curr_task.task_ext().time_stat_output();
    (
        utime_ns / NANOS_PER_SEC as usize,
        utime_ns / NANOS_PER_MICROS as usize,
        stime_ns / NANOS_PER_SEC as usize,
        stime_ns / NANOS_PER_MICROS as usize,
    )
}

pub struct ThreadData {
    /// The clear thread tid field
    ///
    /// See <https://manpages.debian.org/unstable/manpages-dev/set_tid_address.2.en.html#clear_child_tid>
    ///
    /// When the thread exits, the kernel clears the word at this address if it is not NULL.
    pub clear_child_tid: AtomicUsize,
    /// The set thread tid field
    pub set_child_tid: AtomicUsize,

    /// The pending signals
    pub pending: SpinNoIrq<PendingSignals>,
    /// The set of signals currently blocked from delivery.
    pub blocked: Mutex<SignalSet>,
    /// The stack used by signal handlers
    pub signal_stack: Mutex<SignalStack>,
}

impl ThreadData {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            clear_child_tid: AtomicUsize::new(0),
            set_child_tid: AtomicUsize::new(0),
            pending: SpinNoIrq::new(PendingSignals::new()),
            blocked: Mutex::default(),
            signal_stack: Mutex::default(),
        }
    }
}

pub struct ProcessData {
    /// The executable path
    pub exe_path: RwLock<String>,
    /// The virtual memory address space.
    pub aspace: Arc<Mutex<AddrSpace>>,
    /// The resource namespace
    pub ns: AxNamespace,
    /// The user heap bottom
    heap_bottom: AtomicUsize,
    /// The user heap top
    heap_top: AtomicUsize,

    /// The resource limits
    pub rlim: RwLock<Rlimits>,

    /// The process-level shared pending signals
    pub pending: SpinNoIrq<PendingSignals>,
    /// The signal actions
    pub signal_actions: Mutex<[SignalAction; 64]>,
    /// The wait queue for signal. Used by `rt_sigtimedwait`, etc.
    ///
    /// Note that this is shared by all threads in the process, so false wakeups
    /// may occur.
    pub signal_wq: WaitQueue,
    /// The wait queue for child exits.
    pub child_exit_wq: WaitQueue,

    /// The futex table.
    pub futex_table: Mutex<BTreeMap<usize, Arc<WaitQueue>>>,
}

impl ProcessData {
    pub fn new(exe_path: String, aspace: Arc<Mutex<AddrSpace>>) -> Self {
        Self {
            exe_path: RwLock::new(exe_path),
            aspace,
            ns: AxNamespace::new_thread_local(),
            heap_bottom: AtomicUsize::new(axconfig::plat::USER_HEAP_BASE),
            heap_top: AtomicUsize::new(axconfig::plat::USER_HEAP_BASE),

            rlim: RwLock::default(),

            pending: SpinNoIrq::new(PendingSignals::new()),
            signal_actions: Mutex::new(array::from_fn(|_| SignalAction::default())),
            signal_wq: WaitQueue::new(),
            child_exit_wq: WaitQueue::new(),

            futex_table: Mutex::default(),
        }
    }

    pub fn get_heap_bottom(&self) -> usize {
        self.heap_bottom.load(Ordering::Acquire)
    }

    pub fn set_heap_bottom(&self, bottom: usize) {
        self.heap_bottom.store(bottom, Ordering::Release)
    }

    pub fn get_heap_top(&self) -> usize {
        self.heap_top.load(Ordering::Acquire)
    }

    pub fn set_heap_top(&self, top: usize) {
        self.heap_top.store(top, Ordering::Release)
    }
}

impl Drop for ProcessData {
    fn drop(&mut self) {
        if !cfg!(target_arch = "aarch64") && !cfg!(target_arch = "loongarch64") {
            // See [`crate::new_user_aspace`]
            let kernel = kernel_aspace().lock();
            self.aspace
                .lock()
                .clear_mappings(VirtAddrRange::from_start_size(kernel.base(), kernel.size()));
        }
    }
}

struct AxNamespaceImpl;
#[crate_interface::impl_interface]
impl AxNamespaceIf for AxNamespaceImpl {
    fn current_namespace_base() -> *mut u8 {
        // Namespace for kernel task
        static KERNEL_NS_BASE: Once<usize> = Once::new();
        let current = axtask::current();
        // Safety: We only check whether the task extended data is null and do not access it.
        if unsafe { current.task_ext_ptr() }.is_null() {
            return *(KERNEL_NS_BASE.call_once(|| {
                let global_ns = AxNamespace::global();
                let layout = Layout::from_size_align(global_ns.size(), 64).unwrap();
                // Safety: The global namespace is a static readonly variable and will not be dropped.
                let dst = unsafe { alloc::alloc::alloc(layout) };
                let src = global_ns.base();
                unsafe { core::ptr::copy_nonoverlapping(src, dst, global_ns.size()) };
                dst as usize
            })) as *mut u8;
        }
        current.task_ext().process_data().ns.base()
    }
}

static THREAD_TABLE: RwLock<WeakMap<Pid, Weak<Thread>>> = RwLock::new(WeakMap::new());
static PROCESS_TABLE: RwLock<WeakMap<Pid, Weak<Process>>> = RwLock::new(WeakMap::new());
static PROCESS_GROUP_TABLE: RwLock<WeakMap<Pid, Weak<ProcessGroup>>> = RwLock::new(WeakMap::new());
static SESSION_TABLE: RwLock<WeakMap<Pid, Weak<Session>>> = RwLock::new(WeakMap::new());

pub fn add_thread_to_table(thread: &Arc<Thread>) {
    let mut thread_table = THREAD_TABLE.write();
    thread_table.insert(thread.tid(), thread);

    let mut process_table = PROCESS_TABLE.write();
    let process = thread.process();
    if process_table.contains_key(&process.pid()) {
        return;
    }
    process_table.insert(process.pid(), process);

    let mut process_group_table = PROCESS_GROUP_TABLE.write();
    let process_group = process.group();
    if process_group_table.contains_key(&process_group.pgid()) {
        return;
    }
    process_group_table.insert(process_group.pgid(), &process_group);

    let mut session_table = SESSION_TABLE.write();
    let session = process_group.session();
    if session_table.contains_key(&session.sid()) {
        return;
    }
    session_table.insert(session.sid(), &session);
}

pub fn processes() -> Vec<Arc<Process>> {
    PROCESS_TABLE.read().values().collect()
}

pub fn get_thread(tid: Pid) -> LinuxResult<Arc<Thread>> {
    THREAD_TABLE.read().get(&tid).ok_or(LinuxError::ESRCH)
}
pub fn get_process(pid: Pid) -> LinuxResult<Arc<Process>> {
    PROCESS_TABLE.read().get(&pid).ok_or(LinuxError::ESRCH)
}
pub fn get_process_group(pgid: Pid) -> LinuxResult<Arc<ProcessGroup>> {
    PROCESS_GROUP_TABLE
        .read()
        .get(&pgid)
        .ok_or(LinuxError::ESRCH)
}
pub fn get_session(sid: Pid) -> LinuxResult<Arc<Session>> {
    SESSION_TABLE.read().get(&sid).ok_or(LinuxError::ESRCH)
}
