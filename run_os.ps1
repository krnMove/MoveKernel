$ErrorActionPreference = "Stop"

$osImage = Join-Path $PSScriptRoot "os\os.img"
$dataImage = Join-Path $PSScriptRoot "os\data.img"

if (-not (Test-Path $osImage)) {
    throw "Missing $osImage. Run build_os.ps1 first."
}
if (-not (Test-Path $dataImage)) {
    throw "Missing $dataImage. Run build_os.ps1 first."
}

& qemu-system-x86_64 `
    -drive "format=raw,file=$osImage,if=ide,index=0" `
    -drive "format=raw,file=$dataImage,if=ide,index=1"
