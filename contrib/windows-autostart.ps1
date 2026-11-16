# Windows: start PORUMBEL when you log in (no admin rights needed).
#   powershell -ExecutionPolicy Bypass -File contrib\windows-autostart.ps1 "C:\Tools\porumbel.exe"
# Remove it again:  schtasks /Delete /TN PORUMBEL /F
param([Parameter(Mandatory = $true)][string]$Exe)
if (-not (Test-Path -LiteralPath $Exe)) { Write-Error "not found: $Exe"; exit 1 }
$full = (Resolve-Path -LiteralPath $Exe).Path
schtasks /Create /TN PORUMBEL /TR "`"$full`"" /SC ONLOGON /RL LIMITED /F
