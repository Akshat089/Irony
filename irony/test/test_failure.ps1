# =============================================================================
# test_failure.ps1
# Tests failure detection and re-replication
# Run AFTER test_basic.ps1 has successfully written 20 keys
# This script does NOT kill workers automatically — you kill them manually
# Usage: .\scripts\test_failure.ps1
# =============================================================================

$CONTROLLER = "http://127.0.0.1:9090"
$PORTS = @(8081, 8082, 8083, 8084)
$KEYS = @("alpha","bravo","charlie","delta","echo","foxtrot","golf","hotel","india","juliet","kilo","lima","mike","november","oscar","papa","quebec","romeo","sierra","tango")

Write-Host "============================================" -ForegroundColor Cyan
Write-Host " IronRing Failure & Re-replication Test" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# Step 1 — snapshot replica counts before failure
Write-Host "`n[1] Snapshotting replica counts before failure..." -ForegroundColor Yellow
$before = @{}
foreach ($k in $KEYS) {
    $count = 0
    foreach ($port in $PORTS) {
        try {
            Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null
            $count++
        } catch {}
    }
    $before[$k] = $count
}
$allThree = ($before.Values | Where-Object { $_ -eq 3 }).Count
Write-Host "    Keys with 3 replicas before failure: $allThree / $($KEYS.Count)" -ForegroundColor Green

# Step 2 — prompt user to kill a worker
Write-Host "`n[2] MANUAL STEP REQUIRED" -ForegroundColor Yellow
Write-Host "    Kill worker-2 now by pressing Ctrl+C in its terminal." -ForegroundColor Yellow
Write-Host "    Then press Enter here to continue monitoring..." -ForegroundColor Yellow
Read-Host

# Step 3 — monitor until node is marked dead
Write-Host "`n[3] Monitoring controller for failure detection..." -ForegroundColor Yellow
$detected = $false
$attempts = 0

while (-not $detected -and $attempts -lt 20) {
    Start-Sleep 2
    $attempts++
    try {
        $nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
        $deadNodes = $nodes | Where-Object { $_.status -eq "Dead" }
        $suspectNodes = $nodes | Where-Object { $_.status -eq "Suspect" }
        $aliveNodes = $nodes | Where-Object { $_.status -eq "Alive" }

        Write-Host "    [${attempts}] Alive: $($aliveNodes.Count)  Suspect: $($suspectNodes.Count)  Dead: $($deadNodes.Count)"

        if ($deadNodes.Count -gt 0) {
            Write-Host "    Node marked Dead: $($deadNodes.node_id)" -ForegroundColor Red
            $detected = $true
        }
    } catch {
        Write-Host "    Could not reach controller" -ForegroundColor Red
    }
}

if (-not $detected) {
    Write-Host "    Failure not detected within timeout — check heartbeat checker" -ForegroundColor Red
    exit 1
}

# Step 4 — wait for re-replication to complete
Write-Host "`n[4] Waiting 10 seconds for re-replication to complete..." -ForegroundColor Yellow
Start-Sleep 10

# Step 5 — verify all keys still have 3 copies on surviving workers
Write-Host "`n[5] Verifying replica counts after re-replication..." -ForegroundColor Yellow
$survivingPorts = $PORTS | Where-Object { $_ -ne 8082 }
$replicaOk = 0
$replicaWarn = 0

foreach ($k in $KEYS) {
    $count = 0
    foreach ($port in $survivingPorts) {
        try {
            Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null
            $count++
        } catch {}
    }
    if ($count -eq 3) {
        $replicaOk++
    } else {
        Write-Host "    WARN: $k has $count copies on surviving workers (expected 3)" -ForegroundColor Yellow
        $replicaWarn++
    }
}

Write-Host "    Keys with 3 copies after re-replication: $replicaOk / $($KEYS.Count)" -ForegroundColor Green
if ($replicaWarn -gt 0) {
    Write-Host "    Keys needing attention: $replicaWarn" -ForegroundColor Yellow
}

# Step 6 — verify reads still work on surviving workers
Write-Host "`n[6] Verifying reads still work after failure..." -ForegroundColor Yellow
$readOk = 0
foreach ($k in $KEYS) {
    foreach ($port in $survivingPorts) {
        try {
            Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null
            $readOk++
            break
        } catch {}
    }
}
Write-Host "    Keys readable after failure: $readOk / $($KEYS.Count)" -ForegroundColor Green

# Step 7 — verify writes still work after failure
Write-Host "`n[7] Verifying writes still work after failure..." -ForegroundColor Yellow
$written = $false
foreach ($port in $survivingPorts) {
    try {
        $r = Invoke-RestMethod -Method Put `
            -Uri "http://127.0.0.1:$port/v1/keys/post-failure-key" `
            -ContentType "application/json" `
            -Body '{"value":"written-after-failure"}'
        Write-Host "    Write after failure succeeded on port $port (replicas: $($r.replicas_confirmed))" -ForegroundColor Green
        $written = $true
        break
    } catch {}
}
if (-not $written) {
    Write-Host "    Write after failure FAILED" -ForegroundColor Red
}

# Summary
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host " Failure Test Summary" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host " Failure detected        : $(if ($detected) { 'YES' } else { 'NO' })"
Write-Host " Keys re-replicated (3x) : $replicaOk / $($KEYS.Count)"
Write-Host " Reads working           : $readOk / $($KEYS.Count)"
Write-Host " Writes working          : $(if ($written) { 'YES' } else { 'NO' })"

if ($detected -and $replicaOk -eq $KEYS.Count -and $readOk -eq $KEYS.Count -and $written) {
    Write-Host "`n ALL FAILURE TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "`n SOME FAILURE TESTS FAILED — check output above" -ForegroundColor Red
}
