use std::path::PathBuf;

fn main() {
    // Путь теперь от корневого target, а не от kernel/target
    let kernel_path = PathBuf::from("../target/x86_64-unknown-none/debug/my-kernel");
    let out_path = PathBuf::from("os.img");

    bootloader::BiosBoot::new(&kernel_path)
        .create_disk_image(&out_path)
        .expect("Failed to create disk image");

    println!("Created os.img");
}