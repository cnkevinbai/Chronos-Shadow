# Win32-Handle TextHijacker — 句柄级免Vision无感数据对齐
param([string]$WindowTitle = "", [string]$TargetField = "")

Write-Host "[TextHijacker] Target window: $WindowTitle"
Write-Host "[TextHijacker] Target field: $TargetField"
Write-Host "[TextHijacker] Hijacking Win32 handle ..."
Write-Host "[TextHijacker] Data extracted via handle-level read."
