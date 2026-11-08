#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use core::fmt::Write; // Для write!
use bootloader_api::{entry_point, BootInfo};
use x86_64::VirtAddr;
use framebuffer::Writer;

mod framebuffer;
mod interrupts;
mod pic;
mod memory;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // 1. Извлекаем framebuffer и memory_regions до framebuffer::init
    unsafe {
    	FRAMEBUFFER = boot_info.framebuffer.take();
    }
    framebuffer::init(unsafe { FRAMEBUFFER.as_mut() });

    unsafe {
    	if let Some(w) = &mut crate::WRITER {
        	for byte in w.framebuffer.iter_mut() { *byte = 0x00; }
        	w.x_pos = 0;
        	w.y_pos = 0;
        	let _ = write!(w, "MoveKernel 1.2.5 by Move (MoveOS ; MoveKernel) DEV\nMoveKernel> ");
    	}
    }

    print!("Before init_idt\n"); // <-- ПРОВЕРКА

    interrupts::init_idt();
    interrupts::init_pic();
    x86_64::instructions::interrupts::enable();
    print!("\nMoveKernel> ");

    let memory_regions = &boot_info.memory_regions;
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());


    // 2. Инициализация маппинга (memory_regions и phys_mem_offset уже извлечены)
    let mut mapper = unsafe { memory::init_page_table(phys_mem_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(memory_regions) };

    // memory::test_memory(&mut mapper, &mut frame_allocator);

    loop {}
}

static mut FRAMEBUFFER: Option<bootloader_api::info::FrameBuffer> = None;

pub static mut WRITER: Option<framebuffer::Writer> = None;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        use core::fmt::Write;
        unsafe {
            if let Some(w) = &mut $crate::WRITER {
                let _ = write!(w, $($arg)*);
            }
        }
    }};
}