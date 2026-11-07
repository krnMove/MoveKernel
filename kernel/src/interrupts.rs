use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use x86_64::structures::gdt::{GlobalDescriptorTable, Descriptor};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use lazy_static::lazy_static;
use x86_64::structures::idt::PageFaultErrorCode;
use x86_64::instructions::port::Port;
use pic8259::ChainedPics;
use spin;
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::vga_buffer::WRITER;

static TICKS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard, // Это автоматически будет PIC_1_OFFSET + 1 (33)
}

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

// 1. TSS с отдельным стеком для double fault
lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    };
}

// 2. GDT
lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
	let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
	let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

pub struct Selectors {
    code_selector: x86_64::structures::gdt::SegmentSelector,
    tss_selector: x86_64::structures::gdt::SegmentSelector,
}

pub fn init_gdt() {
    GDT.0.load();
    unsafe {
        use x86_64::instructions::segmentation::set_cs;
        set_cs(GDT.1.code_selector);
        x86_64::instructions::tables::load_tss(GDT.1.tss_selector);
    }
}

// 3. IDT (таблица прерываний)
lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt[InterruptIndex::Keyboard as usize].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Timer as usize].set_handler_fn(timer_interrupt_handler);
	unsafe {
    		PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer as u8);
	}
        
        unsafe {
	    idt.page_fault.set_handler_fn(page_fault_handler);
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }
        
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

// 4. Обработчики исключений
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    
    // Пока просто зависаем, чтобы увидеть сообщение
    loop {}
}

extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    
    // Проверяем, что это не отпускание клавиши
    if scancode < 128 {
        if scancode == 0x0E { // Backspace
            WRITER.lock().backspace();
        } else if let Some(c) = SCANCODE_TO_CHAR[scancode as usize] {
            print!("{}", c);
        }
    }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard as u8);
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame)
{
    // Счётчик тиков (если переменная не используется, можно пока убрать)
    let _ticks = TICKS.fetch_add(1, Ordering::Relaxed);
    
    // Мы закомментировали вывод точек, чтобы не мешать клавиатуре
    // if _ticks % 18 == 0 {
    //     print!(".");
    // }

    unsafe {
        PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer as u8);
    }
}

static SCANCODE_TO_CHAR: [Option<char>; 128] = {
    let mut map = [None; 128];
    map[0x02] = Some('1'); map[0x03] = Some('2'); map[0x04] = Some('3');
    map[0x05] = Some('4'); map[0x06] = Some('5'); map[0x07] = Some('6');
    map[0x08] = Some('7'); map[0x09] = Some('8'); map[0x0A] = Some('9');
    map[0x0B] = Some('0'); map[0x0C] = Some('-'); map[0x0D] = Some('=');
    
    map[0x10] = Some('q'); map[0x11] = Some('w'); map[0x12] = Some('e');
    map[0x13] = Some('r'); map[0x14] = Some('t'); map[0x15] = Some('y');
    map[0x16] = Some('u'); map[0x17] = Some('i'); map[0x18] = Some('o');
    map[0x19] = Some('p');
    
    map[0x1E] = Some('a'); map[0x1F] = Some('s'); map[0x20] = Some('d');
    map[0x21] = Some('f'); map[0x22] = Some('g'); map[0x23] = Some('h');
    map[0x24] = Some('j'); map[0x25] = Some('k'); map[0x26] = Some('l');
    
    map[0x2C] = Some('z'); map[0x2D] = Some('x'); map[0x2E] = Some('c');
    map[0x2F] = Some('v'); map[0x30] = Some('b'); map[0x31] = Some('n');
    map[0x32] = Some('m');
    
    map[0x39] = Some(' '); // Пробел
    map
};