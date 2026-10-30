//! Process management syscalls
use core::mem::size_of;

use crate::mm::{test_alloc_num, translated_byte_buffer};
use crate::task::current_user_token;
use crate::timer::get_time_us;
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,syscall_num_find,insert_framed,munmap_framed};
use crate::mm::PageTable;
use crate::mm::VirtAddr;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
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

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
        match trace_request {
        0 => {
            let vaddr = VirtAddr::from(id);
            let vpn = vaddr.floor();
            let page_table = PageTable::from_token(current_user_token());
            if let Some(page_table_entry) = page_table.translate(vpn){
                if page_table_entry.readable() && page_table_entry.is_valid() && page_table_entry.is_userable(){
                    return page_table_entry.ppn().get_bytes_array()[vaddr.page_offset()] as isize;
                }
            }
            -1 as isize
        },
        1 => {
            let vaddr = VirtAddr::from(id);
            let vpn = vaddr.floor();
            let page_table = PageTable::from_token(current_user_token());
            let byte = (data & 0xff) as u8;
            if let Some(page_table_entry) = page_table.translate(vpn){
                if page_table_entry.writable() && page_table_entry.is_valid() && page_table_entry.is_userable(){
                    page_table_entry.ppn().get_bytes_array()[vaddr.page_offset()] = byte;
                    return 0 as isize;
                }
            }
            -1 as isize
        },
        2 => syscall_num_find(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
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
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
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
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
