use font8x8::UnicodeFonts;
use bootloader_api::info::FrameBufferInfo;
use bootloader_api::BootInfo;

pub struct Writer {
    pub framebuffer: &'static mut [u8],
    pub info: FrameBufferInfo,
    pub x_pos: usize,
    pub y_pos: usize,
}

impl Writer {
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        Self {
            framebuffer,
            info,
            x_pos: 0,
            y_pos: 0,
        }
    }

    pub fn clear_screen(&mut self) {
        for byte in self.framebuffer.iter_mut() {
            *byte = 0x00;
        }
        self.x_pos = 0;
        self.y_pos = 0;
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.x_pos + 16 >= self.info.width {
                    self.new_line();
                }

                if let Some(glyph) = font8x8::BASIC_FONTS.get(byte as char) {
                    let x = self.x_pos;
                    let y = self.y_pos;
                    for (row, byte_val) in glyph.iter().enumerate() {
                        for col in 0..8 {
                            if byte_val & (1 << col) != 0 {
                                for dy in 0..2 {
                                    for dx in 0..2 {
                                        let pixel_x = x + col * 2 + dx;
                                        let pixel_y = y + row * 2 + dy;
                                        if pixel_x < self.info.width && pixel_y < self.info.height {
                                            self.draw_pixel(pixel_x, pixel_y, 0xFF, 0xFF, 0xFF);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.x_pos += 16;
            }
        }
    }

    fn new_line(&mut self) {
        self.x_pos = 0;
        self.y_pos += 18;
        if self.y_pos >= self.info.height {
            self.y_pos = 0;
        }
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }
        let offset = (y * self.info.width + x) * self.info.bytes_per_pixel;
        if offset + 2 < self.framebuffer.len() {
            self.framebuffer[offset] = b;
            self.framebuffer[offset + 1] = g;
            self.framebuffer[offset + 2] = r;
        }
    }

    pub fn backspace(&mut self) {
        if self.x_pos >= 16 {
            self.x_pos -= 16;
            let x = self.x_pos;
            let y = self.y_pos;
            for row in 0..16 {
                for col in 0..16 {
                    let pixel_x = x + col;
                    let pixel_y = y + row;
                    if pixel_x < self.info.width && pixel_y < self.info.height {
                        self.draw_pixel(pixel_x, pixel_y, 0x00, 0x00, 0x00);
                    }
                }
            }
        }
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

pub fn init(boot_info: &'static mut BootInfo) {
    if let Some(fb) = boot_info.framebuffer.as_mut() {
        let info = fb.info();
        let buffer = fb.buffer_mut();
        let writer = Writer::new(buffer, info);
        unsafe { crate::WRITER = Some(writer); }
    }
}