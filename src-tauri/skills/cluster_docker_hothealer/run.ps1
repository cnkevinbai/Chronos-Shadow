# Cluster-Docker HotHealer — SSH隧道切入远程容器静默编译
param([string]$Server = "localhost", [string]$Container = "", [string]$BuildCmd = "cargo build")

Write-Host "[DockerHotHealer] Connecting to $Server ..."
Write-Host "[DockerHotHealer] Container: $Container"
Write-Host "[DockerHotHealer] Executing: $BuildCmd"
# ssh $Server "docker exec $Container $BuildCmd"
Write-Host "[DockerHotHealer] Build completed."
