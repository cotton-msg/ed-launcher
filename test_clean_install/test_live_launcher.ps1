$ErrorActionPreference = "Stop"
$testDir = "C:\Users\supminer\Documents\ed-launcher\test_clean_install"
$realMcDir = "$env:USERPROFILE\.minecraft"
$backupDir = "$testDir\backup_real"
$emptyMcDir = "$testDir\empty_minecraft"
$launcherExe = "C:\Users\supminer\Documents\ed-launcher\src-tauri\target\release\grand-eden-launcher.exe"

Write-Host "=== LIVE LAUNCHER TEST (isolated ~/.minecraft) ===" -ForegroundColor Cyan

# Step 1: Backup real ~/.minecraft, replace with empty
if (Test-Path $backupDir) { Remove-Item -Recurse -Force $backupDir }
if (Test-Path $emptyMcDir) { Remove-Item -Recurse -Force $emptyMcDir }

Write-Host "[1] Backing up real ~/.minecraft..." -ForegroundColor Yellow
Copy-Item -Recurse -Force $realMcDir $backupDir
Write-Host "    Backed up to $backupDir"

Write-Host "[2] Replacing ~/.minecraft with empty dir..." -ForegroundColor Yellow
Remove-Item -Recurse -Force $realMcDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $realMcDir | Out-Null

# Step 2: Backup and clear app data
$appDataDir = "$env:APPDATA\ru.grandeden.launcher\game"
$backupAppData = "$testDir\backup_appdata"
if (Test-Path $backupAppData) { Remove-Item -Recurse -Force $backupAppData }
if (Test-Path $appDataDir) {
    Write-Host "[3] Backing up app data..." -ForegroundColor Yellow
    Copy-Item -Recurse -Force $appDataDir $backupAppData
    Remove-Item -Recurse -Force $appDataDir -ErrorAction SilentlyContinue
}

Write-Host "[4] Starting launcher..." -ForegroundColor Yellow
Write-Host "    EXE: $launcherExe"
Write-Host "    EMPTY ~/.minecraft at: $realMcDir"

# Launch the launcher
$proc = Start-Process -FilePath $launcherExe -PassThru -WindowStyle Normal
Write-Host "    Launcher PID: $($proc.Id)"

Write-Host "`n[5] Launcher is running. Monitor ~/.minecraft for downloads..." -ForegroundColor Yellow
Write-Host "    Watching for up to 300 seconds (5 min)..." -ForegroundColor Gray

$startTime = Get-Date
$lastFileCount = 0
while ((Get-Date) - $startTime -lt [TimeSpan]::FromSeconds(300)) {
    Start-Sleep -Seconds 5
    $elapsed = [math]::Round(((Get-Date) - $startTime).TotalSeconds)

    $fileCount = (Get-ChildItem -Path $realMcDir -Recurse -File -ErrorAction SilentlyContinue).Count
    $dirSize = if ($fileCount -gt 0) {
        [math]::Round((Get-ChildItem -Path $realMcDir -Recurse -File -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum / 1MB)
    } else { 0 }

    if ($fileCount -ne $lastFileCount) {
        Write-Host "    [${elapsed}s] Files: $fileCount, Size: ${dirSize}MB" -ForegroundColor Gray
        $lastFileCount = $fileCount
    }

    # Check key files
    $forgeJson = Test-Path "$realMcDir\versions\1.20.1-forge-47.4.20\1.20.1-forge-47.4.20.json"
    $clientJar = Test-Path "$realMcDir\versions\1.20.1-forge-47.4.20\1.20.1-47.4.20.jar"
    $modsExist = Test-Path "$realMcDir\mods"
    $libsExist = Test-Path "$realMcDir\libraries"
    $assetIndex = Test-Path "$realMcDir\assets\indexes\5.json"

    if ($forgeJson -and $clientJar -and $modsExist -and $libsExist -and $assetIndex) {
        Write-Host "`n    ALL KEY FILES PRESENT after ${elapsed}s!" -ForegroundColor Green
        Write-Host "    Forge JSON: OK" -ForegroundColor Green
        Write-Host "    Client JAR: OK" -ForegroundColor Green
        Write-Host "    Mods dir:   OK" -ForegroundColor Green
        Write-Host "    Libraries:  OK" -ForegroundColor Green
        Write-Host "    Asset idx:  OK" -ForegroundColor Green

        $modCount = (Get-ChildItem -Path "$realMcDir\mods" -File -ErrorAction SilentlyContinue).Count
        $libCount = (Get-ChildItem -Path "$realMcDir\libraries" -Recurse -File -ErrorAction SilentlyContinue).Count
        Write-Host "    Mods: $modCount jars" -ForegroundColor Green
        Write-Host "    Libs: $libCount files" -ForegroundColor Green
        break
    }
}

Write-Host "`n[6] Final check:" -ForegroundColor Yellow

# Detailed verification
$checks = @(
    @{ Name = "Forge version JSON"; Path = "$realMcDir\versions\1.20.1-forge-47.4.20\1.20.1-forge-47.4.20.json" },
    @{ Name = "Client JAR"; Path = "$realMcDir\versions\1.20.1-forge-47.4.20\1.20.1-47.4.20.jar" },
    @{ Name = "Vanilla JSON"; Path = "$realMcDir\versions\1.20.1-forge-47.4.20\1.20.1.json" },
    @{ Name = "Asset index 5.json"; Path = "$realMcDir\assets\indexes\5.json" },
    @{ Name = "Mods directory"; Path = "$realMcDir\mods" },
    @{ Name = "Config directory"; Path = "$realMcDir\config" },
    @{ Name = "Libraries directory"; Path = "$realMcDir\libraries" }
)

foreach ($check in $checks) {
    $exists = Test-Path $check.Path
    $icon = if ($exists) { "+" } else { "x" }
    $color = if ($exists) { "Green" } else { "Red" }
    Write-Host "    [$icon] $($check.Name)" -ForegroundColor $color
}

# Check mod files in correct location
if (Test-Path "$realMcDir\mods") {
    $modFiles = Get-ChildItem -Path "$realMcDir\mods" -File
    Write-Host "`n    Mods in ~/.minecraft/mods/:" -ForegroundColor Cyan
    foreach ($mod in $modFiles) {
        Write-Host "      - $($mod.Name) ($([math]::Round($mod.Length/1KB))KB)" -ForegroundColor Gray
    }
}

Write-Host "`n[7] Restoring real ~/.minecraft..." -ForegroundColor Yellow
Remove-Item -Recurse -Force $realMcDir -ErrorAction SilentlyContinue
Copy-Item -Recurse -Force $backupDir $realMcDir
Write-Host "    Restored." -ForegroundColor Green

if (Test-Path $backupAppData) {
    if (-not (Test-Path $appDataDir)) { New-Item -ItemType Directory -Force -Path $appDataDir | Out-Null }
    Copy-Item -Recurse -Force "$backupAppData\*" $appDataDir -ErrorAction SilentlyContinue
    Write-Host "    App data restored." -ForegroundColor Green
}

Write-Host "`n=== TEST COMPLETE ===" -ForegroundColor Cyan
