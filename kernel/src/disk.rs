use x86_64::instructions::port::Port;

const ATA_IO_BASE: u16 = 0x1F0;
const ATA_ALT_STATUS: u16 = 0x3F6;
const DRIVE_SLAVE_LBA: u8 = 0xF0;
const STATUS_ERROR: u8 = 1;
const STATUS_DRQ: u8 = 1 << 3;
const STATUS_DEVICE_FAULT: u8 = 1 << 5;
const STATUS_BUSY: u8 = 1 << 7;
const POLL_LIMIT: usize = 1_000_000;

pub const SECTOR_SIZE: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskError {
    NoDevice,
    Timeout,
    DeviceError,
    InvalidLba,
    DiskTooSmall,
}

pub struct AtaPioDisk {
    sector_count: u32,
}

impl AtaPioDisk {
    pub const fn new() -> Self {
        Self { sector_count: 0 }
    }

    pub fn initialize(&mut self) -> Result<(), DiskError> {
        unsafe {
            Self::select_drive(DRIVE_SLAVE_LBA);
            Self::write_register(2, 0);
            Self::write_register(3, 0);
            Self::write_register(4, 0);
            Self::write_register(5, 0);
            Self::write_register(7, 0xEC);

            let status = Self::read_register(7);
            if status == 0 || status == 0xFF {
                return Err(DiskError::NoDevice);
            }
            Self::wait_not_busy()?;

            if Self::read_register(4) != 0 || Self::read_register(5) != 0 {
                return Err(DiskError::NoDevice);
            }
            Self::wait_data_ready()?;

            let mut sector_count = 0u32;
            for index in 0..256 {
                let word = Self::read_data();
                if index == 60 {
                    sector_count = word as u32;
                } else if index == 61 {
                    sector_count |= (word as u32) << 16;
                }
            }
            if sector_count == 0 {
                return Err(DiskError::NoDevice);
            }
            self.sector_count = sector_count;
        }
        Ok(())
    }

    pub fn sector_count(&self) -> u32 {
        self.sector_count
    }

    pub fn read_sector(
        &mut self,
        lba: u32,
        buffer: &mut [u8; SECTOR_SIZE],
    ) -> Result<(), DiskError> {
        self.check_lba(lba)?;
        unsafe {
            self.begin_sector_transfer(lba, 0x20)?;
            for chunk in buffer.chunks_exact_mut(2) {
                chunk.copy_from_slice(&Self::read_data().to_le_bytes());
            }
        }
        Ok(())
    }

    pub fn write_sector(&mut self, lba: u32, buffer: &[u8; SECTOR_SIZE]) -> Result<(), DiskError> {
        self.check_lba(lba)?;
        unsafe {
            self.begin_sector_transfer(lba, 0x30)?;
            for chunk in buffer.chunks_exact(2) {
                Self::write_data(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
            Self::delay_400ns();
            Self::wait_not_busy()?;
            Self::write_register(7, 0xE7);
            Self::wait_not_busy()?;
            Self::check_status()?;
        }
        Ok(())
    }

    fn check_lba(&self, lba: u32) -> Result<(), DiskError> {
        if self.sector_count == 0 {
            return Err(DiskError::NoDevice);
        }
        if lba >= self.sector_count || lba >= (1 << 28) {
            return Err(DiskError::InvalidLba);
        }
        Ok(())
    }

    unsafe fn begin_sector_transfer(&mut self, lba: u32, command: u8) -> Result<(), DiskError> {
        Self::select_drive(DRIVE_SLAVE_LBA | ((lba >> 24) as u8 & 0x0F));
        Self::delay_400ns();
        Self::wait_not_busy()?;
        Self::write_register(2, 1);
        Self::write_register(3, lba as u8);
        Self::write_register(4, (lba >> 8) as u8);
        Self::write_register(5, (lba >> 16) as u8);
        Self::write_register(7, command);
        Self::wait_data_ready()
    }

    unsafe fn wait_not_busy() -> Result<(), DiskError> {
        for _ in 0..POLL_LIMIT {
            let status = Self::read_register(7);
            if status == 0 || status == 0xFF {
                return Err(DiskError::NoDevice);
            }
            if status & STATUS_BUSY == 0 {
                return Self::check_status();
            }
        }
        Err(DiskError::Timeout)
    }

    unsafe fn wait_data_ready() -> Result<(), DiskError> {
        for _ in 0..POLL_LIMIT {
            let status = Self::read_register(7);
            if status == 0 || status == 0xFF {
                return Err(DiskError::NoDevice);
            }
            if status & STATUS_BUSY == 0 {
                Self::check_status()?;
                if status & STATUS_DRQ != 0 {
                    return Ok(());
                }
            }
        }
        Err(DiskError::Timeout)
    }

    unsafe fn check_status() -> Result<(), DiskError> {
        let status = Self::read_register(7);
        if status & (STATUS_ERROR | STATUS_DEVICE_FAULT) != 0 {
            Err(DiskError::DeviceError)
        } else {
            Ok(())
        }
    }

    unsafe fn select_drive(value: u8) {
        Self::write_register(6, value);
    }

    unsafe fn delay_400ns() {
        let mut status = Port::<u8>::new(ATA_ALT_STATUS);
        for _ in 0..4 {
            let _ = status.read();
        }
    }

    unsafe fn read_register(register: u8) -> u8 {
        Port::<u8>::new(ATA_IO_BASE + register as u16).read()
    }

    unsafe fn write_register(register: u8, value: u8) {
        Port::<u8>::new(ATA_IO_BASE + register as u16).write(value);
    }

    unsafe fn read_data() -> u16 {
        Port::<u16>::new(ATA_IO_BASE).read()
    }

    unsafe fn write_data(value: u16) {
        Port::<u16>::new(ATA_IO_BASE).write(value);
    }
}

pub static DISK: spin::Mutex<AtaPioDisk> = spin::Mutex::new(AtaPioDisk::new());
