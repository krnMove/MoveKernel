use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::{pic, print};
use crate::filesystem::{FileSystemError, FILE_SYSTEM};
use crate::scheduler;
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
    map[0x34] = Some('.');
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
    map[0x34] = Some('>');
    map[0x39] = Some(' '); // Пробел
    map[0x35] = Some('?');
    map
};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = pic::PIC_1_OFFSET,
    Keyboard = pic::PIC_1_OFFSET + 1,
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt[InterruptIndex::Timer as u8].set_handler_fn(timer_interrupt_handler);
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

pub fn init_timer() {
    const PIT_COMMAND: u16 = 0x43;
    const PIT_CHANNEL_0: u16 = 0x40;
    const PIT_FREQUENCY: u32 = 1_193_182;
    const TICK_HZ: u32 = 100;
    let divisor = (PIT_FREQUENCY / TICK_HZ) as u16;

    unsafe {
        let mut command = x86_64::instructions::port::Port::new(PIT_COMMAND);
        let mut channel_0 = x86_64::instructions::port::Port::new(PIT_CHANNEL_0);
        command.write(0x36u8);
        channel_0.write(divisor as u8);
        channel_0.write((divisor >> 8) as u8);
        pic::PICS.enable_timer();
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    unsafe {
        pic::PICS.notify_end_of_interrupt(InterruptIndex::Timer as u8);
    }
    scheduler::on_timer_tick();
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
        if scancode == 0x0E { // Backspace
            unsafe {
                if COMMAND_LENGTH > 0 {
                    COMMAND_LENGTH -= 1;
                    if let Some(w) = &mut crate::WRITER {
                        if COMMAND_BUFFER[COMMAND_LENGTH] == b'\n' {
                            let previous_line_len = COMMAND_BUFFER[..COMMAND_LENGTH]
                                .iter()
                                .rev()
                                .take_while(|byte| **byte != b'\n')
                                .count();
                            w.x_pos = previous_line_len * 16;
                            w.y_pos = w.y_pos.saturating_sub(18);
                        } else if w.x_pos > 0 {
                            w.backspace();
                        }
                    }
                }
            }
        } else if scancode == 0x1C { // Enter
            if SHIFT_PRESSED.load(Ordering::Relaxed) {
                unsafe {
                    if COMMAND_LENGTH < COMMAND_BUFFER_SIZE - 1 {
                        COMMAND_BUFFER[COMMAND_LENGTH] = b'\n';
                        COMMAND_LENGTH += 1;
                        print!("\n");
                    }
                }
            } else {
                unsafe { execute_command(); }
            }
        } else {
            // Символ
            let shift = SHIFT_PRESSED.load(Ordering::Relaxed);
            let caps = CAPS_LOCK_ON.load(Ordering::Relaxed);
            let use_upper = shift ^ caps;
            
            let table = if use_upper {
                &SCANCODE_TO_CHAR_UPPER
            } else {
                &SCANCODE_TO_CHAR_LOWER
            };
            
            if let Some(c) = table[scancode as usize] {
                unsafe {
                    if COMMAND_LENGTH < COMMAND_BUFFER_SIZE - 1 {
                        COMMAND_BUFFER[COMMAND_LENGTH] = c as u8;
                        COMMAND_LENGTH += 1;
                    }
                }
                print!("{}", c); // <-- ВОТ ЭТО!
            }
        }
    }

    unsafe {
        pic::PICS.notify_end_of_interrupt(InterruptIndex::Keyboard as u8);
    }
}

unsafe fn execute_command() {
    let input = core::str::from_utf8_unchecked(&COMMAND_BUFFER[..COMMAND_LENGTH]);
    let mut has_command = false;
    for cmd in input.split('\n').filter(|cmd| !cmd.is_empty()) {
        has_command = true;
        execute_command_line(cmd);
    }
    if !has_command {
        execute_command_line("");
    }
    COMMAND_LENGTH = 0;
}

unsafe fn execute_command_line(cmd: &str) {
    
    // Разделяем команду и аргументы (по первому пробелу)
    let mut parts = cmd.splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");
    
    match command {
        "/clear" => {
            if let Some(w) = &mut crate::WRITER {
                w.clear_screen();
                w.draw_logo();
                w.write_left_aligned_line(crate::KERNEL_TITLE);
            }
            print!("MoveKernel> ");
        }
        "/help" => {
            print!("\nAvailable commands:\n  /clear - clear screen\n  /help - show this help\n  /echo <text> - print text\n  /version - show version\n  /ls - list files\n  /touch <file> - create an empty file\n  /write <file> <text> - replace file contents\n  /cat <file> - show file contents\n  /rm <file> - remove file\n  /spawn - start a counter task\n  /tasks - list running tasks\n  /memtest - test memory read/write\n  /memread <addr> - read memory\n  /memset <addr> <val> - write memory\nShift+Enter starts another command; Enter runs all entered commands.\nMoveKernel> ");
        }
	"/echo" => {
    		print!("\n{}\nMoveKernel> ", args);
	}
        "/heap" => {
            print!("\nHeap is initialized at startup\nMoveKernel> ");
        }
        "/version" => {
            print!("\n{}\nMoveKernel> ", crate::KERNEL_TITLE);
        }
        "" => {
            print!("\nMoveKernel> ");
        }
        "/ls" => {
            let file_system = FILE_SYSTEM.lock();
            print!("\n");
            if !file_system.is_mounted() {
                print!("Filesystem is not mounted\n");
            } else {
                let mut has_files = false;
                for name in file_system.file_names() {
                    print!("{}\n", name);
                    has_files = true;
                }
                if !has_files {
                    print!("(no files)\n");
                }
            }
            print!("MoveKernel> ");
        }
        "/spawn" => match scheduler::spawn_counter() {
            Ok(id) => print!("\nStarted counter task {}\nMoveKernel> ", id),
            Err(scheduler::SpawnError::TaskLimitReached) => {
                print!("\nTask limit reached (maximum 3 counter tasks)\nMoveKernel> ");
            }
        },
        "/tasks" => {
            print!("\nID  TYPE     TICKS       COUNTER\n");
            for id in 0..scheduler::task_count() {
                if let Some(task) = scheduler::task_info(id) {
                    if task.is_kernel {
                        print!("{:<3} kernel   {:<11} -\n", task.id, task.ticks);
                    } else {
                        print!(
                            "{:<3} counter  {:<11} {}\n",
                            task.id, task.ticks, task.work_count
                        );
                    }
                }
            }
            print!("MoveKernel> ");
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
            let name = args.trim();
            if name.is_empty() {
                print!("\nUsage: /touch <file>\nMoveKernel> ");
            } else {
                match FILE_SYSTEM.lock().create_file(name) {
                    Ok(()) => print!("\nCreated file {}\nMoveKernel> ", name),
                    Err(error) => print!("\nCannot create file: {}\nMoveKernel> ", fs_error_message(error)),
                }
            }
        }
        "/cat" => {
            let name = args.trim();
            if name.is_empty() {
                print!("\nUsage: /cat <file>\nMoveKernel> ");
            } else {
                let file_system = FILE_SYSTEM.lock();
                match file_system.read_file(name) {
                    Ok(contents) => print!("\n{}\nMoveKernel> ", contents),
                    Err(error) => print!("\nCannot read file: {}\nMoveKernel> ", fs_error_message(error)),
                }
            }
        }
        "/write" => {
            match args.split_once(' ') {
                Some((name, contents)) if !name.is_empty() => {
                    match FILE_SYSTEM.lock().write_file(name, contents) {
                        Ok(()) => print!("\nWrote file {}\nMoveKernel> ", name),
                        Err(error) => print!("\nCannot write file: {}\nMoveKernel> ", fs_error_message(error)),
                    }
                }
                _ => print!("\nUsage: /write <file> <text>\nMoveKernel> "),
            }
        }
        "/rm" => {
            let name = args.trim();
            if name.is_empty() {
                print!("\nUsage: /rm <file>\nMoveKernel> ");
            } else {
                match FILE_SYSTEM.lock().remove_file(name) {
                    Ok(()) => print!("\nRemoved {}\nMoveKernel> ", name),
                    Err(error) => print!("\nCannot remove file: {}\nMoveKernel> ", fs_error_message(error)),
                }
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
}

fn fs_error_message(error: FileSystemError) -> &'static str {
    match error {
        FileSystemError::Disk(_) => "disk I/O error",
        FileSystemError::CorruptData => "filesystem metadata is corrupt",
        FileSystemError::NotMounted => "filesystem is not mounted; attach data.img",
        FileSystemError::InvalidName => "invalid file name",
        FileSystemError::AlreadyExists => "file already exists",
        FileSystemError::NotFound => "file not found",
        FileSystemError::TooManyFiles => "file limit reached",
        FileSystemError::FileTooLarge => "file is too large (maximum 1024 bytes)",
        FileSystemError::StorageFull => "filesystem storage limit reached",
    }
}