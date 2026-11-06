//! Process management syscalls
use core::mem::size_of;

use crate::mm::{test_alloc_num, translated_byte_buffer};
use crate::task::TaskControlBlock;
use crate::timer::get_time_us;
use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
        insert_framed,munmap_framed
    },
};
use crate::mm::PageTable;
use crate::mm::VirtAddr;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let buf_len = size_of::<TimeVal>();
    let buffers = translated_byte_buffer(
        current_user_token(),
        ts as *const u8,
        buf_len
    );
    let us = get_time_us();
    let time_val = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    
    let src = unsafe {
        core::slice::from_raw_parts(&time_val as *const TimeVal as *const u8, buf_len)
    };
    let mut offset = 0;
    for buffer in buffers {
        let copy_len = buffer.len().min(buf_len - offset);
        buffer[..copy_len].copy_from_slice(&src[offset..offset + copy_len]);
        offset += copy_len;
        if offset >= buf_len {
            break;
        }
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let start_vaddr = VirtAddr::from(start);
    let end_vaddr = VirtAddr::from(start+len);
    let page_table = PageTable::from_token(current_user_token());
    let mut vpn = start_vaddr.floor();
    if start_vaddr.page_offset() != 0 || (port & !(0x7 as usize)) != 0 || (port & 0x7) == 0 {return -1;}
    //println!("start_vaddr:{}  end_vaddr{}",start_vaddr.0,end_vaddr.0);
    println!("map: start_pagenum:{} end_pagenum{}",start_vaddr.floor().0,end_vaddr.floor().0);
    while vpn.0 <= end_vaddr.floor().0 {
        if vpn.0 == end_vaddr.floor().0 && end_vaddr.page_offset() == 0 {break;}
        let pte = page_table.translate(vpn);
        if pte.is_some() && pte.unwrap().is_valid() {
            println!("{} has alloc",vpn.0);
            return -1;
        }
        vpn.0+=1;
    }
    let fact_pagenum = len /4096 + if len % 4096 != 0 {1} else {0};
    if test_alloc_num(fact_pagenum) != 0 {return -1;}
    insert_framed(start_vaddr, end_vaddr, port);
    println!("map: start_pagenum:{} end_pagenum{} finished",start_vaddr.floor().0,end_vaddr.floor().0);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let start_vaddr = VirtAddr::from(start);
    let end_vaddr = VirtAddr::from(start+len);
    let page_table = PageTable::from_token(current_user_token());
    let mut vpn = start_vaddr.floor();
    println!("umap start_pagenum:{} end_pagenum:{}",start_vaddr.floor().0,end_vaddr.floor().0);
    if start_vaddr.page_offset() !=0 {return -1;}
    while vpn.0 <= end_vaddr.floor().0 {
        if vpn.0 == end_vaddr.floor().0 && end_vaddr.page_offset() == 0 {break;}
        let pte = page_table.translate(vpn);
        if pte.is_none() || !pte.unwrap().is_valid() {
            println!("{} has not alloc",vpn.0);
            return -1;
        }
        vpn.0+=1;
    }
    munmap_framed(start_vaddr, end_vaddr);
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let path = translated_str(current_user_token(), path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let child = Arc::new(TaskControlBlock::new(data));
        let mut child_inner = child.inner_exclusive_access();
        let parent = current_task().unwrap();
        let mut parent_inner = parent.inner_exclusive_access();
        child_inner.parent = Some(Arc::downgrade(&parent));
        parent_inner.children.push(Arc::clone(&child));
        add_task(Arc::clone(&child));
        return child.getpid() as isize;
    } 
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if current_task().unwrap().inner_exclusive_access().stride.set_priority(prio as usize) {
        prio as isize
    }
    else {-1}
}
