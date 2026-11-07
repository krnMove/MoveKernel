#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
use core::fmt::Write; // Для write!
use bootloader_api::{entry_point, BootInfo};

mod framebuffer;
mod interrupts;
mod pic;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    framebuffer::init(boot_info);

    unsafe {
        if let Some(w) = &mut crate::WRITER {
            for byte in w.framebuffer.iter_mut() {
                *byte = 0x00;
            }
            w.x_pos = 0;
            w.y_pos = 0;
            let _ = write!(w, "MoveKernel v1.2.0\n> ");
        }
    }

    interrupts::init_idt();
    interrupts::init_pic();
    x86_64::instructions::interrupts::enable();
    loop {}
}

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