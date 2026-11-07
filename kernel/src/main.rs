#![no_std]
#![no_main]
// #![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use bootloader_api::{entry_point, BootInfo};
// mod vga_buffer;
// mod interrupts;
// extern crate alloc;
// mod allocator;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    if let Some(framebuffer) = boot_info.framebuffer.as_mut() {
        let info = framebuffer.info();
        let buffer = framebuffer.buffer_mut();

        // Заливаем экран тёмно-синим цветом (BGR формат)
        for pixel in buffer.iter_mut() {
            *pixel = 0x00_00_88; // Синий
        }
    }

    loop {}
}