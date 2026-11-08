use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::{pic, print};

use core::sync::atomic::{AtomicBool, Ordering};

static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
static CAPS_LOCK_ON: AtomicBool = AtomicBool::new(false);

static SCANCODE_TO_CHAR_LOWER: [Option<char>; 128] = {
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

static SCANCODE_TO_CHAR_UPPER: [Option<char>; 128] = {
    let mut map = [None; 128];
    map[0x02] = Some('!'); map[0x03] = Some('@'); map[0x04] = Some('#');
    map[0x05] = Some('$'); map[0x06] = Some('%'); map[0x07] = Some('^');
    map[0x08] = Some('&'); map[0x09] = Some('*'); map[0x0A] = Some('(');
    map[0x0B] = Some(')'); map[0x0C] = Some('_'); map[0x0D] = Some('+');
    map[0x10] = Some('Q'); map[0x11] = Some('W'); map[0x12] = Some('E');
    map[0x13] = Some('R'); map[0x14] = Some('T'); map[0x15] = Some('Y');
    map[0x16] = Some('U'); map[0x17] = Some('I'); map[0x18] = Some('O');
    map[0x19] = Some('P');
    map[0x1E] = Some('A'); map[0x1F] = Some('S'); map[0x20] = Some('D');
    map[0x21] = Some('F'); map[0x22] = Some('G'); map[0x23] = Some('H');
    map[0x24] = Some('J'); map[0x25] = Some('K'); map[0x26] = Some('L');
    map[0x2C] = Some('Z'); map[0x2D] = Some('X'); map[0x2E] = Some('C');
    map[0x2F] = Some('V'); map[0x30] = Some('B'); map[0x31] = Some('N');
    map[0x32] = Some('M');
    map[0x39] = Some(' '); // Пробел
    map
};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Keyboard = pic::PIC_1_OFFSET + 1,
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt[InterruptIndex::Keyboard as u8].set_handler_fn(keyboard_interrupt_handler);
        idt
    };
}

pub fn init_idt() {
    IDT.load();
}

pub fn init_pic() {
    unsafe { pic::PICS.initialize(); }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    use crate::print;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Обрабатываем Shift (нажатие и отпускание)
    if scancode == 0x2A || scancode == 0x36 { // Левый или правый Shift
        SHIFT_PRESSED.store(true, Ordering::Relaxed);
    } else if scancode == 0x2A | 0x80 || scancode == 0x36 | 0x80 { // Отпускание Shift
        SHIFT_PRESSED.store(false, Ordering::Relaxed);
    }
    // Обрабатываем Caps Lock (только нажатие)
    else if scancode == 0x3A {
        let current = CAPS_LOCK_ON.load(Ordering::Relaxed);
        CAPS_LOCK_ON.store(!current, Ordering::Relaxed);
    }
    // Обрабатываем обычные клавиши (без кодов отпускания)
    else if scancode < 128 {
        if scancode == 0x0E { // Backspace
            unsafe {
                if let Some(w) = &mut crate::WRITER {
                    if w.x_pos > 32 {
                        w.backspace();
                    }
                }
            }
    	} else if scancode == 0x1C { // Enter
    		print!("\n> ");
        } else {
            let shift = SHIFT_PRESSED.load(Ordering::Relaxed);
            let caps = CAPS_LOCK_ON.load(Ordering::Relaxed);
            
            // Если Shift нажат — используем заглавные (или символы с Shift)
            // Если Caps Lock включен — инвертируем регистр для букв
            let use_upper = shift ^ caps; // XOR: Shift или Caps Lock, но не оба
            
            let table = if use_upper {
                &SCANCODE_TO_CHAR_UPPER
            } else {
                &SCANCODE_TO_CHAR_LOWER
            };
            
            if let Some(c) = table[scancode as usize] {
                print!("{}", c);
            }
        }
    }

    unsafe {
        pic::PICS.notify_end_of_interrupt(InterruptIndex::Keyboard as u8);
    }
}