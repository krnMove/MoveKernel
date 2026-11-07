use std::path::PathBuf;
use bootloader::{BiosBoot, BootConfig};

fn main() {
    let kernel_path = PathBuf::from("../target/x86_64-unknown-none/debug/my-kernel");
    let out_path = PathBuf::from("os.img");

    let mut config = BootConfig::default();
    config.frame_buffer_logging = false;

    BiosBoot::new(&kernel_path)
        .set_boot_config(&config)
        .create_disk_image(&out_path)
        .expect("Failed to create disk image");

    println!("Created os.img");
}