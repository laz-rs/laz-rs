param(
    [parameter(Mandatory = $true)][string]$Source,
    [switch]$Parallel = $false,
    [switch]$Release = $false
)

$las_files = Get-ChildItem -Recurse $Source/*.las

$Mode = if ($Release) { "--release" } else { "" }


For ($i = 0; $i -lt $las_files.Length; $i++) {

    $command = "cargo run --release --example check_compression " + $las_files[$i]

    if ($Parallel) {
        $command = "cargo run $Mode --example par_compression --features parallel -- " + $las_files[$i] + " 2>&1"
    }
    else {
        $command = "cargo run $Mode --example check_compression -- " + $las_files[$i] + " 2>&1"
    }
    Write-Host $command

    Write-Host ($i + 1)  " / "  $las_files.Length ": " $las_files[$i]
    $output = Invoke-Expression -Command $command
    if ($lastexitcode -ne 0)
    {
        Write-Host $output
        exit
    }
}


