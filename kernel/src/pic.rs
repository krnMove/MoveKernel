use x86_64::instructions::port::Port;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub struct Pic {
    command_1: Port<u8>,
    data_1: Port<u8>,
    command_2: Port<u8>,
    data_2: Port<u8>,
}

impl Pic {
    pub const fn new() -> Self {
        Self {
            command_1: Port::new(0x20),
            data_1: Port::new(0x21),
            command_2: Port::new(0xA0),
            data_2: Port::new(0xA1),
        }
    }

    pub unsafe fn initialize(&mut self) {
        // Начинаем инициализацию (ICW1)
        self.command_1.write(0x11);
        self.command_2.write(0x11);

        // Задаём смещения (ICW2). Прерывания будут с 32 по 47.
        self.data_1.write(PIC_1_OFFSET);
        self.data_2.write(PIC_2_OFFSET);

        // Настраиваем каскад (ICW3). Slave подключен к линии 2 Master'а.
        self.data_1.write(0x04);
        self.data_2.write(0x02);

        // Режим 8086 (ICW4)
        self.data_1.write(0x01);
        self.data_2.write(0x01);

        // Пока разрешена только клавиатура; таймер включается после инициализации планировщика.
        self.data_1.write(0xFD); // 1111_1101
        self.data_2.write(0xFF); // Slave пока не трогаем
    }

    pub unsafe fn enable_timer(&mut self) {
        self.data_1.write(0xFC); // Разрешаем IRQ0 и IRQ1
    }

    pub unsafe fn notify_end_of_interrupt(&mut self, irq: u8) {
        if irq >= 8 {
            self.command_2.write(0x20); // EOI для Slave
        }
        self.command_1.write(0x20); // EOI для Master
    }
}

pub static mut PICS: Pic = Pic::new();