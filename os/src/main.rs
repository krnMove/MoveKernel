use bootloader::{BiosBoot, BootConfig};
use std::{fs::OpenOptions, path::PathBuf};

fn main() {
    let kernel_path = PathBuf::from("../target/x86_64-unknown-none/debug/my-kernel");
    let out_path = PathBuf::from("os.img");
    let data_disk_path = PathBuf::from("data.img");

    let mut config = BootConfig::default();
    config.frame_buffer_logging = false;

    let data_disk = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&data_disk_path)
        .expect("Failed to open data disk image");
    const DATA_DISK_SIZE: u64 = 1024 * 1024;
    if data_disk
        .metadata()
        .expect("Failed to read data disk metadata")
        .len()
        < DATA_DISK_SIZE
    {
        data_disk
            .set_len(DATA_DISK_SIZE)
            .expect("Failed to size data disk image");
    }

    BiosBoot::new(&kernel_path)
        .set_boot_config(&config)
        .create_disk_image(&out_path)
        .expect("Failed to create disk image");

    println!("Created os.img");
    println!("Preserved data disk image: {}", data_disk_path.display());
}
