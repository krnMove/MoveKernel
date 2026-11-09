# 1. Build the kernel with build-std.
cargo build --manifest-path kernel\Cargo.toml --target x86_64-unknown-none -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

# 2. Check that the kernel was built in the workspace target directory.
if (-not (Test-Path "target\x86_64-unknown-none\debug\my-kernel")) {
    Write-Error "Ядро не собралось!"
    exit 1
}

# 3. Rebuild the boot image and create data.img if it does not exist.
Push-Location os
try {
    cargo run --manifest-path Cargo.toml
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
}
finally {
    Pop-Location
}