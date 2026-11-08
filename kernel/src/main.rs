#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use core::fmt::Write;
use x86_64::VirtAddr;
use bootloader_api::{
    config::{BootloaderConfig, Mapping},
    entry_point, BootInfo,
};

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

mod framebuffer;
mod interrupts;
mod pic;
mod memory;
extern crate alloc;
mod allocator;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    print!("PANIC: {}\n", info);
    loop {
        core::hint::spin_loop();
    }
}

entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    // 1. хуй framebuffera моего парня
    let framebuffer = boot_info.framebuffer.take();

    // 2. сохранение бут инфо торт
    unsafe { BOOT_INFO = Some(boot_info); }

    // 3. ебаный фреймбуфер
    unsafe { FRAMEBUFFER = framebuffer; }
    framebuffer::init(unsafe { FRAMEBUFFER.as_mut() });

    // 4. вывод в нахуй
    unsafe {
        if let Some(w) = &mut crate::WRITER {
            for byte in w.framebuffer.iter_mut() { *byte = 0x00; }
            w.x_pos = 0;
            w.y_pos = 0;
            let _ = write!(w, "MoveKernel 1.3.0 by Move (MoveOS ; MoveKernel) DEV\n");
        }
    }

    // 5. Прерывания
    interrupts::init_idt();
    interrupts::init_pic();

    // 6. Маппинг (извлекаем из BOOT_INFO)
    let boot_info = unsafe { (*core::ptr::addr_of!(BOOT_INFO)).as_ref().unwrap() };
    let memory_regions = &boot_info.memory_regions;
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mut mapper = unsafe { memory::init_page_table(phys_mem_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(memory_regions) };

    // 7. Куча
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("Heap init failed");
    let test = alloc::boxed::Box::new(42);
    print!("Heap test: {}\nMoveKernel> ", test);

    loop {
        x86_64::instructions::interrupts::enable_and_hlt();
    }
}

static mut FRAMEBUFFER: Option<bootloader_api::info::FrameBuffer> = None;

pub static mut WRITER: Option<framebuffer::Writer> = None;

pub static mut BOOT_INFO: Option<&'static mut BootInfo> = None;

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