param(
    [string]$ToolRoot,
    [switch]$Offline,
    [switch]$IncludeDeviceTests
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$androidRoot = Join-Path $projectRoot 'android'
if (-not $ToolRoot) {
    $projectsRoot = Split-Path -Parent (Split-Path -Parent $projectRoot)
    $ToolRoot = Join-Path $projectsRoot 'my-family-app\.sites-runtime\android-tools'
}
$ToolRoot = (Resolve-Path -LiteralPath $ToolRoot).Path
$jdk = Get-ChildItem -LiteralPath $ToolRoot -Directory -Filter 'jdk-21*' | Sort-Object Name -Descending | Select-Object -First 1
$sdkRoot = Join-Path $ToolRoot 'sdk'
$gradleCache = Join-Path $ToolRoot 'gradle-cache'
$gradle = Get-ChildItem -LiteralPath (Join-Path $gradleCache 'wrapper\dists\gradle-8.14.3-all') -Recurse -File -Filter 'gradle.bat' | Select-Object -First 1
if (-not $jdk -or -not $gradle) { throw 'Existing JDK 21 and Gradle 8.14.3 are required. Pass -ToolRoot to their tool folder.' }
foreach ($relative in @('platforms\android-36\android.jar', 'build-tools\36.0.0\apksigner.bat')) {
    if (-not (Test-Path -LiteralPath (Join-Path $sdkRoot $relative))) { throw "Missing Android SDK tool: $relative" }
}

$saved = @{}
foreach ($name in @('JAVA_HOME', 'ANDROID_HOME', 'ANDROID_SDK_ROOT', 'GRADLE_USER_HOME')) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
}
try {
    $env:JAVA_HOME = $jdk.FullName
    $env:ANDROID_HOME = $sdkRoot
    $env:ANDROID_SDK_ROOT = $sdkRoot
    $env:GRADLE_USER_HOME = $gradleCache
    $localRoot = Join-Path $androidRoot '.local'
    New-Item -ItemType Directory -Path $localRoot -Force | Out-Null
    $keystore = Join-Path $localRoot 'debug.keystore'
    if (-not (Test-Path -LiteralPath $keystore)) {
        & (Join-Path $jdk.FullName 'bin\keytool.exe') -genkeypair -keystore $keystore -storepass android -keypass android -alias androiddebugkey -keyalg RSA -keysize 2048 -validity 10000 -dname 'CN=Ultra STT Android Debug,O=Ultra STT,C=KR'
        if ($LASTEXITCODE -ne 0) { throw 'Could not create the app-local debug signing key.' }
    }
    Push-Location $androidRoot
    try {
        $arguments = @('--no-daemon', '--console=plain', ':app:voiceTests', ':app:assembleDebug', ':app:lintDebug')
        if ($IncludeDeviceTests) { $arguments += @(':app:assembleDebugAndroidTest', ':qa:assembleDebug', ':qa:lintDebug') }
        if ($Offline) { $arguments += '--offline' }
        & $gradle.FullName @arguments
        if ($LASTEXITCODE -ne 0) { throw 'Android tests, APK build or lint failed.' }
    } finally { Pop-Location }
    $apk = Join-Path $androidRoot 'app\build\outputs\apk\debug\app-debug.apk'
    if (-not (Test-Path -LiteralPath $apk)) { throw 'The Android APK was not created.' }
    & (Join-Path $sdkRoot 'build-tools\36.0.0\apksigner.bat') verify --verbose --print-certs $apk
    if ($LASTEXITCODE -ne 0) { throw 'APK signature verification failed.' }
    $out = Join-Path $projectRoot 'output\android'
    New-Item -ItemType Directory -Path $out -Force | Out-Null
    $metadata = Get-Content -LiteralPath (Join-Path $androidRoot 'app\build\outputs\apk\debug\output-metadata.json') -Raw | ConvertFrom-Json
    $version = $metadata.elements[0].versionName
    $deliverable = Join-Path $out "Ultra-STT-$version-debug.apk"
    Copy-Item -LiteralPath $apk -Destination $deliverable -Force
    Get-FileHash -LiteralPath $deliverable -Algorithm SHA256 | Format-List
    Write-Output "APK: $deliverable"
} finally {
    foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
}
