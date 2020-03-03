param(
    [parameter(Mandatory=$true)][string]$source
)

$las_files = Get-ChildItem -Recurse $source/*.las


For ($i = 0; $i -lt $las_files.Length; $i++) {
    $command = "cargo run --release --example check_compression " + $las_files[$i]
    Write-Host $i  " / "  $las_files.Length
    Invoke-Expression -ErrorAction Stop $command
    if ($lastexitcode -ne 0) {
        exit
    }
}


