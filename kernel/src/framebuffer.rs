use font8x8::UnicodeFonts;
use bootloader_api::info::FrameBufferInfo;
use bootloader_api::BootInfo;

const GLYPH_WIDTH: usize = 16;
const GLYPH_HEIGHT: usize = 16;
const LINE_HEIGHT: usize = 18;
const LOGO_TOP: usize = 32;
const LOGO_LINE_DELAY: usize = 200_000;
const TITLE_SCREEN_FRACTION: usize = 2;
const SCREEN_FRACTIONS: usize = 3;

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

    pub fn init(framebuffer: Option<&'static mut bootloader_api::info::FrameBuffer>) {
    	if let Some(fb) = framebuffer {
        	let info = fb.info();
        	let buffer = fb.buffer_mut();
        	let writer = Writer::new(buffer, info);
        	unsafe { crate::WRITER = Some(writer); }
    	}
    }

    pub fn clear_screen(&mut self) {
        for byte in self.framebuffer.iter_mut() {
            *byte = 0x00;
        }
        self.x_pos = 0;
        self.y_pos = 0;
    }

    pub fn draw_logo(&mut self) {
        const LOGO_LINES: [&str; 5] = [
            concat!("M   M", " ", " OOO ", " ", "V   V", " ", "EEEEE"),
            concat!("MM MM", " ", "O   O", " ", "V   V", " ", "E    "),
            concat!("M M M", " ", "O   O", " ", "V   V", " ", "EEEE "),
            concat!("M   M", " ", "O   O", " ", " V V ", " ", "E    "),
            concat!("M   M", " ", " OOO ", " ", "  V  ", " ", "EEEEE"),
        ];
        let line_width = LOGO_LINES[0].len() * GLYPH_WIDTH;
        let start_x = self.info.width.saturating_sub(line_width) / 2;

        for (row, line) in LOGO_LINES.iter().enumerate() {
            self.x_pos = start_x;
            self.y_pos = LOGO_TOP + row * LINE_HEIGHT;
            for byte in line.bytes() {
                self.write_byte(byte);
            }
            if row + 1 < LOGO_LINES.len() {
                for _ in 0..LOGO_LINE_DELAY {
                    core::hint::spin_loop();
                }
            }
        }

        let logo_bottom = LOGO_TOP + LOGO_LINES.len() * LINE_HEIGHT;
        let title_y = self.info.height * TITLE_SCREEN_FRACTION / SCREEN_FRACTIONS;
        self.x_pos = 0;
        self.y_pos = logo_bottom + title_y.saturating_sub(logo_bottom) / 2;
        for _ in 0..self.info.width / GLYPH_WIDTH {
            self.write_byte(b'-');
        }

        self.x_pos = 0;
        self.y_pos = title_y;
    }

    pub fn write_left_aligned_line(&mut self, text: &str) {
        self.x_pos = 0;
        for byte in text.bytes() {
            self.write_byte(byte);
        }
        self.x_pos = 0;
        self.y_pos += LINE_HEIGHT;
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.x_pos + GLYPH_WIDTH > self.info.width {
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

                self.x_pos += GLYPH_WIDTH;
            }
        }
    }

    fn new_line(&mut self) {
        self.x_pos = 0;
        self.y_pos += LINE_HEIGHT;
        if self.y_pos.saturating_add(GLYPH_HEIGHT) > self.info.height {
            self.scroll_up(LINE_HEIGHT);
            self.y_pos = self.y_pos.saturating_sub(LINE_HEIGHT);
        }
    }

    fn scroll_up(&mut self, rows: usize) {
        let rows = rows.min(self.info.height);
        let bytes_per_row = self.info.stride * self.info.bytes_per_pixel;
        let image_size = bytes_per_row * self.info.height;
        if rows == 0 || image_size > self.framebuffer.len() {
            return;
        }

        let scroll_size = bytes_per_row * rows;
        self.framebuffer
            .copy_within(scroll_size..image_size, 0);
        self.framebuffer[image_size - scroll_size..image_size].fill(0);
    }

    pub fn draw_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        if x >= self.info.width || y >= self.info.height {
            return;
        }
        let offset = (y * self.info.stride + x) * self.info.bytes_per_pixel;
        if offset + 2 < self.framebuffer.len() {
            self.framebuffer[offset] = b;
            self.framebuffer[offset + 1] = g;
            self.framebuffer[offset + 2] = r;
        }
    }

    pub fn backspace(&mut self) {
        if self.x_pos >= GLYPH_WIDTH {
            self.x_pos -= GLYPH_WIDTH;
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

pub fn init(framebuffer: Option<&'static mut bootloader_api::info::FrameBuffer>) {
    if let Some(fb) = framebuffer {
        let info = fb.info();
        let buffer = fb.buffer_mut();
        let writer = Writer::new(buffer, info);
        unsafe { crate::WRITER = Some(writer); }
    }
}