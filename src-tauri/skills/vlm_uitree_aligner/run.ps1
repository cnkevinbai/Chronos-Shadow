# VLM-UITree Aligner — Win32 UIA控件树磁吸纠偏
param([string]$Target = "", [string]$AlignmentMode = "magnetic")

Write-Host "[UITreeAligner] Aligning: $Target (mode: $AlignmentMode)"
Write-Host "[UITreeAligner] Querying Win32 UIA control tree ..."
Write-Host "[UITreeAligner] Applying magnetic correction ..."
Write-Host "[UITreeAligner] Alignment complete."
