param(
    [parameter(Mandatory=$true)][string]$source
)

$laz_files = Get-ChildItem -Recurse $source/*.laz
$las_files = Get-ChildItem -Recurse $source/*.las

if ($laz_files.Length -ne $las_files.Length) {
   Throw "There must be as many LAS files as LAZ"
}

For ($i = 0; $i -lt $laz_files.Length; $i++) {
    $command = "cargo run --release --example check_decompression " +  $laz_files[$i] + " " + $las_files[$i]
    Write-Host $i  " / "  $laz_files.Length
    Invoke-Expression -ErrorAction Stop $command
    if ($lastexitcode -ne 0) {
        exit
    }
}


