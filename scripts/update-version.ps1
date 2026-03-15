# Update version in Cargo.toml
# Usage: ./scripts/update-version.ps1 <version|major|minor|patch>
#   version: exact semver e.g. "1.2.3"
#   major: bump major (0.3.4 -> 1.0.0)
#   minor: bump minor (0.3.4 -> 0.4.0)
#   patch: bump patch (0.3.4 -> 0.3.5)

# Only use first argument (version or major/minor/patch); ignore rest
$VersionArg = $args[0]
if (-not $VersionArg) {
    Write-Error "Usage: ./scripts/update-version.ps1 <version|major|minor|patch>"
    exit 1
}

$ErrorActionPreference = "Stop"
$CargoPath = Join-Path (Join-Path $PSScriptRoot "..") "Cargo.toml"
$CargoPath = [System.IO.Path]::GetFullPath($CargoPath)

if (-not (Test-Path $CargoPath)) {
    Write-Error "Cargo.toml not found at $CargoPath"
    exit 1
}

$content = Get-Content $CargoPath -Raw
$versionMatch = [regex]::Match($content, '(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"')
if (-not $versionMatch.Success) {
    Write-Error "Could not find version in Cargo.toml"
    exit 1
}

$current = $versionMatch.Groups[1].Value
$major = [int]($current -split '\.')[0]
$minor = [int]($current -split '\.')[1]
$patch = [int]($current -split '\.')[2]

$newVersion = switch ($VersionArg.ToLower()) {
    "major" { "$($major + 1).0.0" }
    "minor" { "$major.$($minor + 1).0" }
    "patch" { "$major.$minor.$($patch + 1)" }
    default {
        if ($VersionArg -match '^\d+\.\d+\.\d+$') {
            $VersionArg
        } else {
            Write-Error "Invalid version or bump: '$VersionArg'. Use a semver (e.g. 1.2.3) or: major, minor, patch"
            exit 1
        }
    }
}

# Replace only the first occurrence (package version), not dependency versions
$oldLine = $versionMatch.Value
$newLine = "version = `"$newVersion`""
$firstIdx = $content.IndexOf($oldLine)
$newContent = $content.Substring(0, $firstIdx) + $newLine + $content.Substring($firstIdx + $oldLine.Length)

Set-Content -Path $CargoPath -Value $newContent -NoNewline
Write-Host "Updated version: $current -> $newVersion (Cargo.toml)"
