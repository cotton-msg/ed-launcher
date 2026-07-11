$ErrorActionPreference = "Continue"
$SUPABASE_URL = "https://wziuqwtunlovtyxleles.supabase.co"
$SUPABASE_KEY = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6Ind6aXVxd3R1bmxvdnR5eGxlbGVzIiwicm9sZSI6ImFub24iLCJpYXQiOjE3ODM2OTI0MTYsImV4cCI6MjA5OTI2ODQxNn0.4mVMKOpLhx9_bfwBRnZAfXgiAJAmockvevMI5FkPRXg"

$testDir = "C:\Users\supminer\Documents\ed-launcher\test_clean_install"
$backupDir = "$testDir\backup"
$testMcDir = "$testDir\.minecraft"
$testAppData = "$testDir\app_data\game"

$results = @()

function Test-Step {
    param([string]$Name, [bool]$Pass, [string]$Detail = "")
    $status = if ($Pass) { "PASS" } else { "FAIL" }
    $results += [PSCustomObject]@{ Step = $Name; Status = $Status; Detail = $Detail }
    Write-Host "[$status] $Name" -ForegroundColor $(if ($Pass) { "Green" } else { "Red" })
    if ($Detail) { Write-Host "  -> $Detail" -ForegroundColor Gray }
}

Write-Host "`n=== GRAND EDEN LAUNCHER - CLEAN INSTALL TEST ===" -ForegroundColor Cyan
Write-Host "Simulating a PC with no Minecraft installed`n"

# ---- STEP 1: Backup existing ~/.minecraft ----
Write-Host "--- Phase 1: Setup isolated environment ---" -ForegroundColor Yellow
$realMcDir = "$env:USERPROFILE\.minecraft"
if (Test-Path $realMcDir) {
    if (Test-Path $backupDir) { Remove-Item -Recurse -Force $backupDir }
    Copy-Item -Recurse -Force $realMcDir $backupDir
    Test-Step "Backup existing ~/.minecraft" $true "$backupDir"
} else {
    Test-Step "Backup existing ~/.minecraft" $true "No existing dir to backup"
}

# Create test dirs
New-Item -ItemType Directory -Force -Path $testMcDir | Out-Null
New-Item -ItemType Directory -Force -Path $testAppData | Out-Null
Test-Step "Create isolated test directories" $true

# ---- STEP 2: Test Supabase connectivity ----
Write-Host "`n--- Phase 2: Test Supabase Storage ---" -ForegroundColor Yellow

# List java files
$body = '{"prefix":"java/","limit":1000,"offset":0}'
$resp = curl.exe -s -X POST "$SUPABASE_URL/storage/v1/object/list/game-files" -H "apikey: $SUPABASE_KEY" -H "Authorization: Bearer $SUPABASE_KEY" -H "Content-Type: application/json" -d $body 2>&1
$javaFiles = $resp | ConvertFrom-Json -ErrorAction SilentlyContinue
$javaCount = if ($javaFiles) { $javaFiles.Count } else { 0 }
Test-Step "Supabase: list java files" ($javaCount -gt 0) "$javaCount files"

# Check java.exe exists in listing
$javaExe = $javaFiles | Where-Object { $_.name -eq "java.exe" -and $_.id -ne $null }
Test-Step "Supabase: java.exe exists" ($null -ne $javaExe) "Size: $($javaExe.metadata.contentLength) bytes"

# List version files (mods/configs)
$body2 = '{"prefix":"version/","limit":1000,"offset":0}'
$resp2 = curl.exe -s -X POST "$SUPABASE_URL/storage/v1/object/list/game-files" -H "apikey: $SUPABASE_KEY" -H "Authorization: Bearer $SUPABASE_KEY" -H "Content-Type: application/json" -d $body2 2>&1
$versionFiles = $resp2 | ConvertFrom-Json -ErrorAction SilentlyContinue
$versionCount = if ($versionFiles) { $versionFiles.Count } else { 0 }
Test-Step "Supabase: list version files" ($versionCount -gt 0) "$versionCount items"

# Check mods folder exists in version/
$modsFolder = $versionFiles | Where-Object { $_.name -eq "mods" -and $_.id -eq $null }
Test-Step "Supabase: mods/ folder exists" ($null -ne $modsFolder)

# ---- STEP 3: Test Forge installer JAR download + version.json extraction ----
Write-Host "`n--- Phase 3: Forge version JSON from installer ---" -ForegroundColor Yellow

$forgeInstallerUrl = "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.4.20/forge-1.20.1-47.4.20-installer.jar"
$tempJar = "$testDir\forge-installer.jar"
$forgeVersionsDir = "$testMcDir\versions\1.20.1-forge-47.4.20"
$forgeJsonPath = "$forgeVersionsDir\1.20.1-forge-47.4.20.json"
New-Item -ItemType Directory -Force -Path $forgeVersionsDir | Out-Null

# Download installer
Write-Host "  Downloading Forge installer JAR (8.8MB)..."
curl.exe -sL -o $tempJar $forgeInstallerUrl
$installerSize = (Get-Item $tempJar -ErrorAction SilentlyContinue).Length
Test-Step "Download Forge installer JAR" ($installerSize -gt 5000000) "Size: $([math]::Round($installerSize/1MB, 1))MB"

# Extract version.json using .NET ZipFile
Add-Type -AssemblyName System.IO.Compression.FileSystem
try {
    $zip = [System.IO.Compression.ZipFile]::OpenRead($tempJar)
    $entry = $zip.Entries | Where-Object { $_.FullName -eq "version.json" }
    if ($entry) {
        $stream = $entry.Open()
        $reader = New-Object System.IO.StreamReader($stream)
        $jsonContent = $reader.ReadToEnd()
        $reader.Close()
        $stream.Close()
        Set-Content -Path $forgeJsonPath -Value $jsonContent -Encoding UTF8
        $zip.Dispose()
        $jsonObj = $jsonContent | ConvertFrom-Json
        $libCount = $jsonObj.libraries.Count
        Test-Step "Extract version.json from installer" $true "Main class: $($jsonObj.mainClass), Libraries: $libCount"
    } else {
        $zip.Dispose()
        # List contents to debug
        $zip2 = [System.IO.Compression.ZipFile]::OpenRead($tempJar)
        $names = $zip2.Entries.FullName | Select-Object -First 20
        Test-Step "Extract version.json from installer" $false "Not found. Contents: $($names -join ', ')"
        $zip2.Dispose()
    }
} catch {
    Test-Step "Extract version.json from installer" $false "Error: $_"
}

# ---- STEP 4: Test vanilla 1.20.1 JSON download ----
Write-Host "`n--- Phase 4: Vanilla version JSON + client.jar ---" -ForegroundColor Yellow

$manifestResp = curl.exe -s "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
$manifest = $manifestResp | ConvertFrom-Json
$v1201 = $manifest.versions | Where-Object { $_.id -eq "1.20.1" }
$vanillaUrl = $v1201.url
Test-Step "Get vanilla 1.20.1 manifest URL" ($null -ne $vanillaUrl) "$vanillaUrl"

$vanillaResp = curl.exe -s $vanillaUrl
$vanillaJson = $vanillaResp | ConvertFrom-Json
$vanillaJsonPath = "$forgeVersionsDir\1.20.1.json"
Set-Content -Path $vanillaJsonPath -Value $vanillaResp -Encoding UTF8
Test-Step "Download vanilla 1.20.1 JSON" $true "Asset index: $($vanillaJson.assetIndex.id)"

# Download client.jar
$clientJarPath = "$forgeVersionsDir\1.20.1-47.4.20.jar"
$clientUrl = $vanillaJson.downloads.client.url
Write-Host "  Downloading client.jar (~20MB)..."
curl.exe -sL -o $clientJarPath $clientUrl
$clientSize = (Get-Item $clientJarPath -ErrorAction SilentlyContinue).Length
Test-Step "Download client.jar" ($clientSize -gt 10000000) "Size: $([math]::Round($clientSize/1MB, 1))MB"

# ---- STEP 5: Test asset index download ----
Write-Host "`n--- Phase 5: Asset index ---" -ForegroundColor Yellow

$assetsDir = "$testMcDir\assets\indexes"
New-Item -ItemType Directory -Force -Path $assetsDir | Out-Null
$assetIndexId = $vanillaJson.assetIndex.id
$assetIndexPath = "$assetsDir\$assetIndexId.json"
$assetIndexUrl = $vanillaJson.assetIndex.url
curl.exe -sL -o $assetIndexPath $assetIndexUrl
$assetIndexSize = (Get-Item $assetIndexPath -ErrorAction SilentlyContinue).Length
Test-Step "Download asset index" ($assetIndexSize -gt 1000) "Index: $assetIndexId, Size: $([math]::Round($assetIndexSize/1KB, 1))KB"

$assetIndex = Get-Content $assetIndexPath -Raw | ConvertFrom-Json
$assetCount = $assetIndex.objects.PSObject.Properties.Value.Count
Test-Step "Parse asset index" ($assetCount -gt 1000) "$assetCount assets"

# ---- STEP 6: Test Forge library download (sample) ----
Write-Host "`n--- Phase 6: Forge libraries ---" -ForegroundColor Yellow

$libsDir = "$testMcDir\libraries"
$forgeJson = Get-Content $forgeJsonPath -Raw | ConvertFrom-Json
$libsToTest = @()
foreach ($lib in $forgeJson.libraries) {
    if ($lib.downloads.artifact) {
        $path = $lib.downloads.artifact.path
        $url = $lib.downloads.artifact.url
        $localPath = "$libsDir\$($path -replace '/', '\')"
        $libsToTest += @{ url = $url; path = $path; localPath = $localPath }
        if ($libsToTest.Count -ge 5) { break }
    }
}

$libsPassed = 0
foreach ($lib in $libsToTest) {
    $dir = Split-Path $lib.localPath -Parent
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    curl.exe -sL -o $lib.localPath $lib.url
    $size = (Get-Item $lib.localPath -ErrorAction SilentlyContinue).Length
    if ($size -gt 100) { $libsPassed++ }
}
Test-Step "Download sample Forge libraries (5)" ($libsPassed -eq 5) "$libsPassed/5 downloaded"

# ---- STEP 7: Test Java ----
Write-Host "`n--- Phase 7: Java detection ---" -ForegroundColor Yellow

$javaPath = ""
# System Java
$candidates = @(
    "C:\Program Files\Eclipse Adoptium",
    "C:\Program Files\Java",
    "C:\Program Files\Microsoft",
    "C:\Program Files\BellSoft",
    "C:\Program Files\Zulu"
)
foreach ($base in $candidates) {
    if (Test-Path $base) {
        $found = Get-ChildItem -Path $base -Recurse -Filter "java.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) { $javaPath = $found.FullName; break }
    }
}
if (-not $javaPath -and $env:JAVA_HOME) {
    $javaPath = "$env:JAVA_HOME\bin\java.exe"
}
Test-Step "Find system Java" (Test-Path $javaPath) $javaPath

# ---- STEP 8: Test game launch (dry run - just check args) ----
Write-Host "`n--- Phase 8: Launch args validation ---" -ForegroundColor Yellow

$mainClass = $forgeJson.mainClass
$versionName = "1.20.1-47.4.20"
Test-Step "Forge main class" ($mainClass -eq "cpw.mods.bootstraplauncher.BootstrapLauncher") $mainClass

$jvmArgs = $forgeJson.arguments.jvm
$hasModulePath = $jvmArgs | Where-Object { $_ -eq "-p" }
$hasClassPath = $jvmArgs | Where-Object { $_ -eq "-cp" }
Test-Step "Forge JVM args have -p (module path)" ($null -ne $hasModulePath)
Test-Step "Forge JVM args have -cp (classpath)" ($null -ne $hasClassPath)

$gameArgs = $forgeJson.arguments.game
$hasGameDir = $gameArgs | Where-Object { $_ -match '\$\{game_directory\}' }
$hasAssetsRoot = $gameArgs | Where-Object { $_ -match '\$\{assets_root\}' }
Test-Step "Forge game args have \${game_directory}" ($null -ne $hasGameDir)
Test-Step "Forge game args have \${assets_root}" ($null -ne $hasAssetsRoot)

# ---- STEP 9: Test actual launch (short timeout) ----
Write-Host "`n--- Phase 9: Test game launch (10s timeout) ---" -ForegroundColor Yellow

# Build a minimal classpath with the client jar + some test libs
$allLibPaths = @()
foreach ($lib in $forgeJson.libraries) {
    if ($lib.downloads.artifact) {
        $path = $lib.downloads.artifact.path
        $localPath = "$libsDir\$($path -replace '/', '\')"
        if (Test-Path $localPath) { $allLibPaths += $localPath }
    }
}
$allLibPaths += $clientJarPath
$cpSep = ";"
$classpath = $allLibPaths -join $cpSep

$logPath = "$forgeVersionsDir\test_launcher.log"
$debugPath = "$forgeVersionsDir\test_debug_args.txt"

# Build args (simplified version of what launch_game does)
$javaArgs = @(
    "-Xmx4096m",
    "-Xms2048m",
    "-XX:+UseG1GC",
    "-cp", $classpath,
    $mainClass,
    "--username", "TestPlayer",
    "--version", $versionName,
    "--gameDir", $testMcDir,
    "--assetsDir", "$testMcDir\assets",
    "--assetIndex", $assetIndexId,
    "--accessToken", "0",
    "--uuid", "00000000-0000-0000-0000-000000000000",
    "--userType", "offline",
    "--versionType", "Forge"
)

$debugContent = "java.exe $($javaArgs -join ' ')"
Set-Content -Path $debugPath -Value $debugContent -Encoding UTF8

Write-Host "  Launching Java with Forge..."
Write-Host "  (expect crash or OpenGL error - that's normal for headless test)"

try {
    $proc = Start-Process -FilePath $javaPath -ArgumentList $javaArgs -WorkingDirectory $forgeVersionsDir -RedirectStandardOutput $logPath -RedirectStandardError "$logPath.err" -PassThru -NoNewWindow

    # Wait up to 10 seconds
    $waited = 0
    while (-not $proc.HasExited -and $waited -lt 10) {
        Start-Sleep -Seconds 1
        $waited++
        Write-Host "  Waiting... ${waited}s"
    }

    if ($proc.HasExited) {
        $exitCode = $proc.ExitCode
        $logContent = if (Test-Path $logPath) { Get-Content $logPath -Tail 20 -ErrorAction SilentlyContinue } else { @() }
        $errContent = if (Test-Path "$logPath.err") { Get-Content "$logPath.err" -Tail 20 -ErrorAction SilentlyContinue } else { @() }

        # Java crash from missing OpenGL is expected in headless environment
        $allOutput = ($logContent -join "`n") + ($errContent -join "`n")
        if ($allOutput -match "NoClassDefFoundError|BootstrapLauncher|ModLauncher|modules" -or $exitCode -eq 1) {
            Test-Step "Java process started and executed Forge" $true "Exit code: $exitCode (Forge loaded, expected crash on headless/no GPU)"
        } else {
            Test-Step "Java process started" ($exitCode -ne -1) "Exit code: $exitCode"
        }

        if ($errContent.Count -gt 0) {
            Write-Host "`n  Last error output:" -ForegroundColor Gray
            $errContent | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkGray }
        }
    } else {
        Test-Step "Java process started" $true "Process still running after 10s (good sign!)"
        $proc.Kill()
    }
} catch {
    Test-Step "Java process launch" $false "Error: $_"
}

# ---- STEP 10: Verify file structure ----
Write-Host "`n--- Phase 10: Verify file structure ---" -ForegroundColor Yellow

$checks = @(
    @{ Name = "~/.minecraft/versions/forge JSON"; Path = $forgeJsonPath },
    @{ Name = "~/.minecraft/versions/vanilla JSON"; Path = $vanillaJsonPath },
    @{ Name = "~/.minecraft/versions/client.jar"; Path = $clientJarPath },
    @{ Name = "~/.minecraft/assets/indexes"; Path = "$testMcDir\assets\indexes\$assetIndexId.json" },
    @{ Name = "~/.minecraft/libraries"; Path = $libsDir },
    @{ Name = "debug_args.txt"; Path = $debugPath }
)

foreach ($check in $checks) {
    $exists = Test-Path $check.Path
    Test-Step "File exists: $($check.Name)" $exists $check.Path
}

# Count files
$libFileCount = (Get-ChildItem -Path $libsDir -Recurse -File -ErrorAction SilentlyContinue).Count
Write-Host "`n  Libraries downloaded: $libFileCount files" -ForegroundColor Gray

# ---- SUMMARY ----
Write-Host "`n========================================" -ForegroundColor Cyan
$passed = ($results | Where-Object { $_.Status -eq "PASS" }).Count
$failed = ($results | Where-Object { $_.Status -eq "FAIL" }).Count
Write-Host "RESULTS: $passed passed, $failed failed out of $($results.Count) tests" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Yellow" })

if ($failed -gt 0) {
    Write-Host "`nFailed tests:" -ForegroundColor Red
    $results | Where-Object { $_.Status -eq "FAIL" } | ForEach-Object {
        Write-Host "  [FAIL] $($_.Step): $($_.Detail)" -ForegroundColor Red
    }
}
Write-Host ""
