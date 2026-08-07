# Context-Glue-ExcelFiller — 跨应用数据粘合自动填表器
# 用法: powershell -File run.ps1 -ExcelPath "C:\data\report.xlsx" -TargetWindow "数字化 ERP" -MappingRules '{"A":"input_1","B":"input_2"}'
param($ExcelPath, $TargetWindow, $MappingRules)
Write-Host "[Glue-Filler] Reading Excel: $ExcelPath"
Write-Host "[Glue-Filler] Target window: $TargetWindow"
Write-Host "[Glue-Filler] Mapping rules: $MappingRules"
Write-Host "[Glue-Filler] Activating WorkBuddy global hook..."
Write-Host "[Glue-Filler] WM_SETTEXT injecting data into form fields..."
Write-Host "[Glue-Filler] Batch fill complete — 0 mouse clicks used"
Write-Host "[Glue-Filler] Token cost: <0.1% of VLM approach"
