#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;
#[macro_use]
mod vga_buffer;
mod interrupts;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    vga_buffer::WRITER.lock().clear_screen();
    println!("Hello from my-kernel!");
    println!("VGA Writer is working.");
    unsafe { interrupts::PICS.lock().initialize() };
    interrupts::init_idt();
    interrupts::init_gdt();
    println!("IDT loaded.");
    x86_64::instructions::interrupts::enable();
    x86_64::instructions::interrupts::int3();
    println!("Returned from breakpoint!");

    loop {}
}