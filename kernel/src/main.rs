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

fn kernel_main(_boot_info: &'static mut BootInfo) -> ! {
    // vga_buffer::WRITER.lock().clear_screen();
    // println!("Hello from my-kernel!");
    // println!("VGA Writer is working.");
    // unsafe { interrupts::PICS.lock().initialize() };
    // interrupts::init_idt();
    // interrupts::init_gdt();
    // println!("IDT loaded.");
    // x86_64::instructions::interrupts::enable();
    // x86_64::instructions::interrupts::int3();
    // println!("Returned from breakpoint!");

    // allocator::init_heap();
    // println!("Heap initialized.");

    // use alloc::boxed::Box;
    // let heap_test = Box::new(42);
    // println!("Box value: {}", heap_test);

    loop {}
}