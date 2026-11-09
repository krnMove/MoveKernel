use alloc::{string::String, vec::Vec};

use crate::disk::{DiskError, DISK, SECTOR_SIZE};

const MAX_FILES: usize = 32;
const MAX_FILE_NAME_LEN: usize = 24;
const MAX_FILE_CONTENT_LEN: usize = 1024;
const MAX_TOTAL_CONTENT_LEN: usize = 16 * 1024;
const METADATA_SECTORS: u32 = 2;
const DATA_SECTORS_PER_FILE: u32 = 2;
const DISK_SECTORS_REQUIRED: u32 = METADATA_SECTORS + MAX_FILES as u32 * DATA_SECTORS_PER_FILE;
const METADATA_SIZE: usize = METADATA_SECTORS as usize * SECTOR_SIZE;
const HEADER_SIZE: usize = 16;
const ENTRY_SIZE: usize = 28;
const MAGIC: &[u8; 8] = b"MOVEFS01";

struct File {
    name: String,
    contents: String,
    slot: usize,
}

pub struct FileSystem {
    files: Vec<File>,
    total_content_len: usize,
    mounted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileSystemError {
    Disk(DiskError),
    CorruptData,
    NotMounted,
    InvalidName,
    AlreadyExists,
    NotFound,
    TooManyFiles,
    FileTooLarge,
    StorageFull,
}

impl FileSystem {
    pub const fn new() -> Self {
        Self {
            files: Vec::new(),
            total_content_len: 0,
            mounted: false,
        }
    }

    pub fn mount(&mut self) -> Result<(), FileSystemError> {
        let mut disk = DISK.lock();
        disk.initialize().map_err(FileSystemError::Disk)?;
        if disk.sector_count() < DISK_SECTORS_REQUIRED {
            return Err(FileSystemError::Disk(DiskError::DiskTooSmall));
        }

        let mut metadata = [0; METADATA_SIZE];
        for sector_index in 0..METADATA_SECTORS as usize {
            let mut sector = [0; SECTOR_SIZE];
            disk.read_sector(sector_index as u32, &mut sector)
                .map_err(FileSystemError::Disk)?;
            let start = sector_index * SECTOR_SIZE;
            metadata[start..start + SECTOR_SIZE].copy_from_slice(&sector);
        }

        if metadata.iter().all(|byte| *byte == 0) {
            metadata[..MAGIC.len()].copy_from_slice(MAGIC);
            metadata[8..10].copy_from_slice(&1u16.to_le_bytes());
            metadata[10..12].copy_from_slice(&(MAX_FILES as u16).to_le_bytes());
            Self::write_metadata_to_disk(&mut disk, &metadata)?;
            self.files.clear();
            self.total_content_len = 0;
            self.mounted = true;
            return Ok(());
        }

        if &metadata[..MAGIC.len()] != MAGIC
            || u16::from_le_bytes([metadata[8], metadata[9]]) != 1
            || u16::from_le_bytes([metadata[10], metadata[11]]) != MAX_FILES as u16
        {
            return Err(FileSystemError::CorruptData);
        }

        let mut files = Vec::new();
        let mut total_content_len = 0;
        for slot in 0..MAX_FILES {
            let entry = HEADER_SIZE + slot * ENTRY_SIZE;
            match metadata[entry] {
                0 => continue,
                1 => {}
                _ => return Err(FileSystemError::CorruptData),
            }

            let name_bytes = &metadata[entry + 1..entry + 1 + MAX_FILE_NAME_LEN];
            let name_len = name_bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(MAX_FILE_NAME_LEN);
            let name = core::str::from_utf8(&name_bytes[..name_len])
                .map_err(|_| FileSystemError::CorruptData)?;
            Self::validate_name(name).map_err(|_| FileSystemError::CorruptData)?;
            if files.iter().any(|file: &File| file.name == name) {
                return Err(FileSystemError::CorruptData);
            }

            let content_len =
                u16::from_le_bytes([metadata[entry + 25], metadata[entry + 26]]) as usize;
            if content_len > MAX_FILE_CONTENT_LEN {
                return Err(FileSystemError::CorruptData);
            }
            total_content_len += content_len;
            if total_content_len > MAX_TOTAL_CONTENT_LEN {
                return Err(FileSystemError::CorruptData);
            }

            let mut content_bytes = [0; MAX_FILE_CONTENT_LEN];
            for sector_index in 0..DATA_SECTORS_PER_FILE as usize {
                let mut sector = [0; SECTOR_SIZE];
                let lba =
                    METADATA_SECTORS + slot as u32 * DATA_SECTORS_PER_FILE + sector_index as u32;
                disk.read_sector(lba, &mut sector)
                    .map_err(FileSystemError::Disk)?;
                let start = sector_index * SECTOR_SIZE;
                content_bytes[start..start + SECTOR_SIZE].copy_from_slice(&sector);
            }
            let contents = String::from_utf8(content_bytes[..content_len].to_vec())
                .map_err(|_| FileSystemError::CorruptData)?;
            files.push(File {
                name: String::from(name),
                contents,
                slot,
            });
        }

        self.files = files;
        self.total_content_len = total_content_len;
        self.mounted = true;
        Ok(())
    }

    pub fn create_file(&mut self, name: &str) -> Result<(), FileSystemError> {
        self.ensure_mounted()?;
        Self::validate_name(name)?;
        if self.files.iter().any(|file| file.name == name) {
            return Err(FileSystemError::AlreadyExists);
        }
        if self.files.len() >= MAX_FILES {
            return Err(FileSystemError::TooManyFiles);
        }

        let slot = (0..MAX_FILES)
            .find(|slot| self.files.iter().all(|file| file.slot != *slot))
            .ok_or(FileSystemError::TooManyFiles)?;
        self.files.push(File {
            name: String::from(name),
            contents: String::new(),
            slot,
        });
        if let Err(error) = self.persist_metadata() {
            self.files.pop();
            return Err(error);
        }
        Ok(())
    }

    pub fn read_file(&self, name: &str) -> Result<&str, FileSystemError> {
        self.ensure_mounted()?;
        self.files
            .iter()
            .find(|file| file.name == name)
            .map(|file| file.contents.as_str())
            .ok_or(FileSystemError::NotFound)
    }

    pub fn write_file(&mut self, name: &str, contents: &str) -> Result<(), FileSystemError> {
        self.ensure_mounted()?;
        Self::validate_name(name)?;
        if contents.len() > MAX_FILE_CONTENT_LEN {
            return Err(FileSystemError::FileTooLarge);
        }

        let index = self
            .files
            .iter()
            .position(|file| file.name == name)
            .ok_or(FileSystemError::NotFound)?;
        let new_total = self.total_content_len - self.files[index].contents.len() + contents.len();
        if new_total > MAX_TOTAL_CONTENT_LEN {
            return Err(FileSystemError::StorageFull);
        }

        let slot = self.files[index].slot;
        Self::write_contents_to_disk(slot, contents)?;
        let old_contents =
            core::mem::replace(&mut self.files[index].contents, String::from(contents));
        let old_total = self.total_content_len;
        self.total_content_len = new_total;
        if let Err(error) = self.persist_metadata() {
            self.files[index].contents = old_contents;
            self.total_content_len = old_total;
            return Err(error);
        }
        Ok(())
    }

    pub fn remove_file(&mut self, name: &str) -> Result<(), FileSystemError> {
        self.ensure_mounted()?;
        let index = self
            .files
            .iter()
            .position(|file| file.name == name)
            .ok_or(FileSystemError::NotFound)?;
        let file = self.files.remove(index);
        self.total_content_len -= file.contents.len();
        if let Err(error) = self.persist_metadata() {
            self.total_content_len += file.contents.len();
            self.files.insert(index, file);
            return Err(error);
        }
        Ok(())
    }

    pub fn file_names(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(|file| file.name.as_str())
    }

    pub fn is_mounted(&self) -> bool {
        self.mounted
    }

    fn ensure_mounted(&self) -> Result<(), FileSystemError> {
        if self.mounted {
            Ok(())
        } else {
            Err(FileSystemError::NotMounted)
        }
    }

    fn persist_metadata(&self) -> Result<(), FileSystemError> {
        let mut metadata = [0; METADATA_SIZE];
        metadata[..MAGIC.len()].copy_from_slice(MAGIC);
        metadata[8..10].copy_from_slice(&1u16.to_le_bytes());
        metadata[10..12].copy_from_slice(&(MAX_FILES as u16).to_le_bytes());
        for file in &self.files {
            let entry = HEADER_SIZE + file.slot * ENTRY_SIZE;
            metadata[entry] = 1;
            metadata[entry + 1..entry + 1 + file.name.len()].copy_from_slice(file.name.as_bytes());
            metadata[entry + 25..entry + 27]
                .copy_from_slice(&(file.contents.len() as u16).to_le_bytes());
        }

        let mut disk = DISK.lock();
        Self::write_metadata_to_disk(&mut disk, &metadata)
    }

    fn write_metadata_to_disk(
        disk: &mut crate::disk::AtaPioDisk,
        metadata: &[u8; METADATA_SIZE],
    ) -> Result<(), FileSystemError> {
        for sector_index in 0..METADATA_SECTORS as usize {
            let start = sector_index * SECTOR_SIZE;
            let mut sector = [0; SECTOR_SIZE];
            sector.copy_from_slice(&metadata[start..start + SECTOR_SIZE]);
            disk.write_sector(sector_index as u32, &sector)
                .map_err(FileSystemError::Disk)?;
        }
        Ok(())
    }

    fn write_contents_to_disk(slot: usize, contents: &str) -> Result<(), FileSystemError> {
        let mut data = [0; MAX_FILE_CONTENT_LEN];
        data[..contents.len()].copy_from_slice(contents.as_bytes());
        let mut disk = DISK.lock();
        for sector_index in 0..DATA_SECTORS_PER_FILE as usize {
            let start = sector_index * SECTOR_SIZE;
            let mut sector = [0; SECTOR_SIZE];
            sector.copy_from_slice(&data[start..start + SECTOR_SIZE]);
            let lba = METADATA_SECTORS + slot as u32 * DATA_SECTORS_PER_FILE + sector_index as u32;
            disk.write_sector(lba, &sector)
                .map_err(FileSystemError::Disk)?;
        }
        Ok(())
    }

    fn validate_name(name: &str) -> Result<(), FileSystemError> {
        if name.is_empty()
            || name.len() > MAX_FILE_NAME_LEN
            || name
                .chars()
                .any(|character| character.is_whitespace() || character == '/' || character == '\\')
        {
            return Err(FileSystemError::InvalidName);
        }
        Ok(())
    }
}

pub static FILE_SYSTEM: spin::Mutex<FileSystem> = spin::Mutex::new(FileSystem::new());
