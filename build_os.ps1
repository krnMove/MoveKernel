# 1. Собираем ядро с build-std
cargo build --manifest-path kernel/Cargo.toml --target x86_64-unknown-none -Z build-std=core -Z build-std-features=compiler-builtins-mem

# 2. Проверяем, что ядро собралось
if (-not (Test-Path "kernel/target/x86_64-unknown-none/debug/my-kernel")) {
    Write-Error "Ядро не собралось!"
    exit 1
}

# 3. Запускаем отдельный маленький Rust-скрипт, который создаст образ
# (для этого можно использовать os, но уже без build.rs, просто как библиотеку)