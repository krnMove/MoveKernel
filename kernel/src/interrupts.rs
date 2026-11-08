use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::{pic, print};
const COMMAND_BUFFER_SIZE: usize = 128;
static mut COMMAND_BUFFER: [u8; COMMAND_BUFFER_SIZE] = [0; COMMAND_BUFFER_SIZE];
static mut COMMAND_LENGTH: usize = 0;

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
    map[0x35] = Some('/');
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
    map[0x35] = Some('?');
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
    print!("PIC initialized\n"); // <-- Проверка
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;
    use crate::print;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Обработка Shift
    if scancode == 0x2A || scancode == 0x36 {
        SHIFT_PRESSED.store(true, Ordering::Relaxed);
    } else if scancode == (0x2A | 0x80) || scancode == (0x36 | 0x80) {
        SHIFT_PRESSED.store(false, Ordering::Relaxed);
    }
    // Обработка Caps Lock
    else if scancode == 0x3A {
        let current = CAPS_LOCK_ON.load(Ordering::Relaxed);
        CAPS_LOCK_ON.store(!current, Ordering::Relaxed);
    }
    // Обычные клавиши
    else if scancode < 128 {
        if scancode == 0x0E { /* Backspace */ }
        else if scancode == 0x1C { /* Enter */ }
        else {
            // символ
        }
    }

    unsafe {
        pic::PICS.notify_end_of_interrupt(InterruptIndex::Keyboard as u8);
    }
}

unsafe fn execute_command() {
    let cmd = core::str::from_utf8_unchecked(&COMMAND_BUFFER[..COMMAND_LENGTH]);
    
    // Разделяем команду и аргументы (по первому пробелу)
    let mut parts = cmd.splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");
    
    match command {
        "/clear" => {
   		if let Some(w) = &mut crate::WRITER {
        		w.clear_screen();
    		}
    		print!("MoveKernel 1.2.5 by Move (MoveOS ; MoveKernel) DEV\nMoveKernel> ");
	}
        "/help" => {
    		print!("\nAvailable commands:\n  /clear - clear screen\n  /help - show this help\n  /echo <text> - print text\n  /version - show version\n  /ls - list files\n  /pwd - print working directory\n  /cd <dir> - change directory\n  /mkdir <dir> - create directory\n  /touch <file> - create file\n  /cat <file> - show file contents\n  /rm <file> - remove file\n  /memtest - test memory read/write\n  /memread <addr> - read memory\n  /memset <addr> <val> - write memory\nMoveKernel> ");
	}
	"/echo" => {
    		print!("\n{}\nMoveKernel> ", args);
	}
        "/version" => {
            print!("\nMoveKernel v1.2.5\n> ");
        }
        "" => {
    		print!("\nMoveKernel> ");
	}
	"/ls" => {
    		print!("\nfile1.txt  file2.txt  dir1/\nMoveKernel> ");
	}
	"/pwd" => {
    		print!("\n/\nMoveKernel> ");
	}
	"/cd" => {
    		if args.is_empty() {
        		print!("\nUsage: /cd <dir>\nMoveKernel> ");
    		} else {
        		print!("\nChanged directory to {}\nMoveKernel> ", args);
    		}
	}
	"/mkdir" => {
    		if args.is_empty() {
        		print!("\nUsage: /mkdir <dir>\nMoveKernel> ");
    		} else {
        		print!("\nCreated directory {}\nMoveKernel> ", args);
    		}
	}
	"/touch" => {
    		if args.is_empty() {
        		print!("\nUsage: /touch <file>\nMoveKernel> ");
    		} else {
        		print!("\nCreated file {}\nMoveKernel> ", args);
    		}
	}
	"/cat" => {
    		if args.is_empty() {
        		print!("\nUsage: /cat <file>\nMoveKernel> ");
    		} else {
        		print!("\nNo such file: {}\nMoveKernel> ", args);
    		}
	}
	"/rm" => {
    		if args.is_empty() {
        		print!("\nUsage: /rm <file>\nMoveKernel> ");
    		} else {
        		print!("\nRemoved {}\nMoveKernel> ", args);
    		}
	}
	"/memtest" => {
    		// Тест записи/чтения по адресу кучи
    		let test_addr = 0x_4444_4444_0000 as *mut u64;
    		unsafe {
        		test_addr.write_volatile(0xDEADBEEF);
        		let value = test_addr.read_volatile();
        		print!("\nMemory test: wrote 0xDEADBEEF, read 0x{:X}\nMoveKernel> ", value);
    		}
	}
	"/memread" => {
    		if args.is_empty() {
        		print!("\nUsage: /memread <hex_address>\nMoveKernel> ");
    		} else {
        		let addr = u64::from_str_radix(args, 16).unwrap_or(0);
        		let ptr = addr as *const u64;
        		unsafe {
            			let value = ptr.read_volatile();
            			print!("\nMemory at 0x{:X}: 0x{:X}\nMoveKernel> ", addr, value);
        		}		
    		}
	}
	"/memset" => {
    		// Разделяем аргументы на адрес и значение
    		let mut parts = args.splitn(2, ' ');
    		let addr_str = parts.next().unwrap_or("");
    		let val_str = parts.next().unwrap_or("");
    		if addr_str.is_empty() || val_str.is_empty() {
        		print!("\nUsage: /memset <hex_address> <hex_value>\nMoveKernel> ");
    		} else {
        		let addr = u64::from_str_radix(addr_str, 16).unwrap_or(0);
        		let val = u64::from_str_radix(val_str, 16).unwrap_or(0);
        		let ptr = addr as *mut u64;
        		unsafe {
            			ptr.write_volatile(val);
            			print!("\nMemory at 0x{:X} set to 0x{:X}\nMoveKernel> ", addr, val);
        		}
    		}
	}
	_ => {
    		print!("\nUnknown command: {}\nMoveKernel> ", cmd);
	}
    }
    
    // Сбрасываем буфер
    COMMAND_LENGTH = 0;
}