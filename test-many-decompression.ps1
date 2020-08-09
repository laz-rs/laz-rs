param(
    [parameter(Mandatory=$true)][string]$source,
    [switch]$Parallel = $false,
    [switch]$Release = $false
)

$laz_files = Get-ChildItem -Recurse $source/*.laz
$las_files = Get-ChildItem -Recurse $source/*.las

if ($laz_files.Length -ne $las_files.Length) {
   Throw "There must be as many LAS files as LAZ"
}

$Mode = if ($Release) { "--release" } else { "" }

For ($i = 0; $i -lt $laz_files.Length; $i++) {

    if ($Parallel) {
        $command = "cargo run $Mode --example par_decompression --features parallel -- " + $laz_files[$i] + " " + $las_files[$i] + " 2>&1"
    } else {
        $command = "cargo run $Mode --example check_decompression -- " + $laz_files[$i] + " " + $las_files[$i] + " 2>&1"
    }
    Write-Host $command
    Write-Host ($i + 1)  " / "  $laz_files.Length ": " $laz_files[$i] " vs " $las_files[$i]
    $output = Invoke-Expression -Command $command
    if ($lastexitcode -ne 0) {
        Write-Host $output
        exit
    }
}


