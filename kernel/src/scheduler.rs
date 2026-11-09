use alloc::{boxed::Box, vec, vec::Vec};
use core::{
    arch::global_asm,
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_TASKS: usize = 4;
const TASK_STACK_SIZE: usize = 16 * 1024;

global_asm!(
    ".global context_switch",
    "context_switch:",
    "push rbx",
    "push rbp",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    "mov [rdi], rsp",
    "mov rsp, rsi",
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop rbp",
    "pop rbx",
    "ret",
);

unsafe extern "C" {
    fn context_switch(current_stack: *mut usize, next_stack: usize);
}

struct Task {
    saved_stack: usize,
    _stack: Option<Box<[u8]>>,
    ticks: AtomicU64,
    work_count: Option<Box<AtomicU64>>,
}

struct Scheduler {
    tasks: Vec<Task>,
    current: usize,
}

static mut SCHEDULER: Scheduler = Scheduler {
    tasks: Vec::new(),
    current: 0,
};

#[derive(Clone, Copy)]
pub struct TaskInfo {
    pub id: usize,
    pub is_kernel: bool,
    pub ticks: u64,
    pub work_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnError {
    TaskLimitReached,
}

pub fn init() {
    unsafe {
        let scheduler = core::ptr::addr_of_mut!(SCHEDULER);
        (*scheduler).tasks.push(Task {
            saved_stack: 0,
            _stack: None,
            ticks: AtomicU64::new(0),
            work_count: None,
        });
        (*scheduler).current = 0;
    }
}

pub fn spawn_counter() -> Result<usize, SpawnError> {
    unsafe {
        let scheduler = core::ptr::addr_of_mut!(SCHEDULER);
        if (*scheduler).tasks.len() >= MAX_TASKS {
            return Err(SpawnError::TaskLimitReached);
        }

        let mut stack = vec![0; TASK_STACK_SIZE].into_boxed_slice();
        let stack_top = (stack.as_mut_ptr() as usize + stack.len()) & !0xF;
        let initial_stack = stack_top - 8 * 8;
        let stack_words = initial_stack as *mut usize;
        for index in 0..6 {
            stack_words.add(index).write(0);
        }
        stack_words
            .add(6)
            .write(task_bootstrap as *const () as usize);
        stack_words.add(7).write(0);

        (*scheduler).tasks.push(Task {
            saved_stack: initial_stack,
            _stack: Some(stack),
            ticks: AtomicU64::new(0),
            work_count: Some(Box::new(AtomicU64::new(0))),
        });
        Ok((*scheduler).tasks.len() - 1)
    }
}

pub fn task_count() -> usize {
    unsafe { (*core::ptr::addr_of!(SCHEDULER)).tasks.len() }
}

pub fn task_info(id: usize) -> Option<TaskInfo> {
    unsafe {
        let scheduler = &*core::ptr::addr_of!(SCHEDULER);
        let task = scheduler.tasks.get(id)?;
        Some(TaskInfo {
            id,
            is_kernel: id == 0,
            ticks: task.ticks.load(Ordering::Relaxed),
            work_count: task
                .work_count
                .as_ref()
                .map_or(0, |count| count.load(Ordering::Relaxed)),
        })
    }
}

pub fn on_timer_tick() {
    unsafe {
        let scheduler = core::ptr::addr_of_mut!(SCHEDULER);
        let tasks = (*scheduler).tasks.as_mut_ptr();
        let task_count = (*scheduler).tasks.len();
        if task_count == 0 {
            return;
        }
        let current = (*scheduler).current;
        (*tasks.add(current)).ticks.fetch_add(1, Ordering::Relaxed);
        if task_count < 2 {
            return;
        }

        let next = (current + 1) % task_count;
        (*scheduler).current = next;

        let current_stack = core::ptr::addr_of_mut!((*tasks.add(current)).saved_stack);
        let next_stack = (*tasks.add(next)).saved_stack;
        context_switch(current_stack, next_stack);
    }
}

extern "C" fn task_bootstrap() -> ! {
    let counter = unsafe {
        let scheduler = &*core::ptr::addr_of!(SCHEDULER);
        let task = &scheduler.tasks[scheduler.current];
        task.work_count
            .as_ref()
            .expect("counter task has no counter")
            .as_ref() as *const AtomicU64
    };
    let counter = unsafe { &*counter };

    x86_64::instructions::interrupts::enable();
    loop {
        counter.fetch_add(1, Ordering::Relaxed);
        core::hint::spin_loop();
    }
}
