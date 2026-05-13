# =============================================================================
# test_recovery.ps1
# Tests worker recovery after failure
# Run AFTER test_failure.ps1 — with worker-2 still dead
# Usage: .\scripts\test_recovery.ps1
# =============================================================================

$CONTROLLER = "http://127.0.0.1:9090"
$PORTS = @(8081, 8082, 8083, 8084)

Write-Host "============================================" -ForegroundColor Cyan
Write-Host " IronRing Recovery Test" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# Step 1 — confirm worker-2 is still dead
Write-Host "`n[1] Confirming pre-recovery state..." -ForegroundColor Yellow
$nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
$deadNodes = $nodes | Where-Object { $_.status -eq "Dead" }
Write-Host "    Dead nodes: $($deadNodes.Count)"
$deadNodes | ForEach-Object { Write-Host "    - $($_.node_id)" -ForegroundColor Red }

$ringBefore = Invoke-RestMethod -Uri "$CONTROLLER/v1/ring"
$versionBefore = $ringBefore.ring_version
Write-Host "    Ring version before recovery: $versionBefore"
Write-Host "    Nodes in ring before recovery: $($ringBefore.nodes.Count)"

# Step 2 — prompt user to restart worker
Write-Host "`n[2] MANUAL STEP REQUIRED" -ForegroundColor Yellow
Write-Host "    Restart worker-2 in its terminal:" -ForegroundColor Yellow
Write-Host '    $env:NODE_ID="worker-2"; $env:NODE_PORT="8082"; $env:HOST="127.0.0.1"; $env:CONTROLLER_ADDR="http://127.0.0.1:9090"; cargo run -p worker' -ForegroundColor White
Write-Host "    Then press Enter here to continue monitoring..." -ForegroundColor Yellow
Read-Host

# Step 3 — monitor until worker-2 is alive again
Write-Host "`n[3] Monitoring for recovery..." -ForegroundColor Yellow
$recovered = $false
$attempts = 0

while (-not $recovered -and $attempts -lt 15) {
    Start-Sleep 2
    $attempts++
    try {
        $nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
        $aliveNodes = $nodes | Where-Object { $_.status -eq "Alive" }
        $deadNodes = $nodes | Where-Object { $_.status -eq "Dead" }
        Write-Host "    [${attempts}] Alive: $($aliveNodes.Count)  Dead: $($deadNodes.Count)"
        if ($aliveNodes.Count -eq 4) {
            Write-Host "    All 4 nodes alive again!" -ForegroundColor Green
            $recovered = $true
        }
    } catch {
        Write-Host "    Could not reach controller" -ForegroundColor Red
    }
}

if (-not $recovered) {
    Write-Host "    Recovery not detected — check worker-2 terminal" -ForegroundColor Red
    exit 1
}

# Step 4 — verify ring version incremented
Write-Host "`n[4] Verifying ring updated after recovery..." -ForegroundColor Yellow
$ringAfter = Invoke-RestMethod -Uri "$CONTROLLER/v1/ring"
$versionAfter = $ringAfter.ring_version
Write-Host "    Ring version before: $versionBefore"
Write-Host "    Ring version after : $versionAfter"
Write-Host "    Nodes in ring      : $($ringAfter.nodes.Count)"

if ($versionAfter -gt $versionBefore) {
    Write-Host "    Ring version incremented correctly." -ForegroundColor Green
} else {
    Write-Host "    WARNING: Ring version did not increment." -ForegroundColor Yellow
}

# Step 5 — confirm recovered worker starts empty
Write-Host "`n[5] Confirming recovered worker starts with empty store..." -ForegroundColor Yellow
try {
    $dump = Invoke-RestMethod -Uri "http://127.0.0.1:8082/v1/keys"
    $keyCount = ($dump.keys | Get-Member -MemberType NoteProperty).Count
    Write-Host "    Worker-2 key count: $keyCount"
    if ($keyCount -eq 0) {
        Write-Host "    Correct — recovered worker starts empty." -ForegroundColor Green
        Write-Host "    NOTE: This is expected. Worker-2 will receive new writes going forward." -ForegroundColor Cyan
    } else {
        Write-Host "    Worker-2 has $keyCount keys (unexpected)" -ForegroundColor Yellow
    }
} catch {
    Write-Host "    Could not reach worker-2" -ForegroundColor Red
}

# Step 6 — confirm writes go to recovered worker
Write-Host "`n[6] Writing new keys and checking distribution..." -ForegroundColor Yellow
$newKeys = @("recovery-1","recovery-2","recovery-3","recovery-4","recovery-5")
foreach ($k in $newKeys) {
    foreach ($port in $PORTS) {
        try {
            Invoke-RestMethod -Method Put `
                -Uri "http://127.0.0.1:$port/v1/keys/$k" `
                -ContentType "application/json" `
                -Body "{`"value`":`"value-$k`"}" | Out-Null
            break
        } catch {}
    }
}

Start-Sleep 2

# Check if any new keys landed on the recovered worker
$worker2Keys = Invoke-RestMethod -Uri "http://127.0.0.1:8082/v1/keys"
$worker2Count = ($worker2Keys.keys | Get-Member -MemberType NoteProperty -ErrorAction SilentlyContinue).Count
Write-Host "    Worker-2 now holds $worker2Count keys after new writes." -ForegroundColor Green

# Summary
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host " Recovery Test Summary" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host " Recovery detected       : $(if ($recovered) { 'YES' } else { 'NO' })"
Write-Host " Ring version incremented: $(if ($versionAfter -gt $versionBefore) { 'YES' } else { 'NO' })"
Write-Host " Ring has 4 nodes        : $(if ($ringAfter.nodes.Count -eq 4) { 'YES' } else { 'NO' })"
Write-Host " Worker-2 keys after new writes: $worker2Count"

if ($recovered -and $versionAfter -gt $versionBefore -and $ringAfter.nodes.Count -eq 4) {
    Write-Host "`n ALL RECOVERY TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "`n SOME RECOVERY TESTS FAILED — check output above" -ForegroundColor Red
}
