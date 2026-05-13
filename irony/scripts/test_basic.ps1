# =============================================================================
# test_basic.ps1
# Tests basic PUT and GET operations across all workers
# Run this after all 5 nodes are up and healthy
# Usage: .\scripts\test_basic.ps1
# =============================================================================

$CONTROLLER = "http://127.0.0.1:9090"
$PORTS = @(8081, 8082, 8083, 8084)
$KEYS = @("alpha","bravo","charlie","delta","echo","foxtrot","golf","hotel","india","juliet","kilo","lima","mike","november","oscar","papa","quebec","romeo","sierra","tango")

Write-Host "============================================" -ForegroundColor Cyan
Write-Host " IronRing Basic Test" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# Step 1 - confirm all nodes are alive
Write-Host "`n[1] Checking cluster health..." -ForegroundColor Yellow
$nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
$aliveCount = ($nodes | Where-Object { $_.status -eq "Alive" }).Count
Write-Host "    Alive nodes: $aliveCount / $($nodes.Count)"
if ($aliveCount -lt 4) {
    Write-Host "    ERROR: Not all nodes are alive. Start all workers first." -ForegroundColor Red
    exit 1
}
Write-Host "    All nodes alive." -ForegroundColor Green

# Step 2 - confirm ring is populated
Write-Host "`n[2] Checking ring state..." -ForegroundColor Yellow
$ring = Invoke-RestMethod -Uri "$CONTROLLER/v1/ring"
Write-Host "    Nodes in ring: $($ring.nodes.Count)"
Write-Host "    Virtual nodes: $($ring.virtual_nodes.Count)"
if ($ring.nodes.Count -lt 4) {
    Write-Host "    ERROR: Ring does not have all 4 workers." -ForegroundColor Red
    exit 1
}
Write-Host "    Ring is healthy." -ForegroundColor Green

# Step 3 - PUT all 20 keys
Write-Host "`n[3] Writing 20 keys..." -ForegroundColor Yellow
$putSuccess = 0
$putFailed = 0

foreach ($k in $KEYS) {
    $written = $false
    foreach ($port in $PORTS) {
        try {
            $r = Invoke-RestMethod -Method Put `
                -Uri "http://127.0.0.1:$port/v1/keys/$k" `
                -ContentType "application/json" `
                -Body "{`"value`":`"value-$k`"}"
            $putSuccess++
            $written = $true
            break
        } catch {}
    }
    if (-not $written) {
        Write-Host "    FAILED to write key: $k" -ForegroundColor Red
        $putFailed++
    }
}

Write-Host "    Written: $putSuccess / $($KEYS.Count)" -ForegroundColor Green
if ($putFailed -gt 0) {
    Write-Host "    Failed: $putFailed" -ForegroundColor Red
}

# Step 4 - GET all keys back and verify values
Write-Host "`n[4] Reading all keys back..." -ForegroundColor Yellow
$getSuccess = 0
$getFailed = 0

foreach ($k in $KEYS) {
    $found = $false
    foreach ($port in $PORTS) {
        try {
            $r = Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k"
            if ($r.value -eq "value-$k") {
                $found = $true
                break
            }
        } catch {}
    }
    if ($found) { $getSuccess++ }
    else {
        Write-Host "    MISSING or wrong value for key: $k" -ForegroundColor Red
        $getFailed++
    }
}

Write-Host "    Found correct: $getSuccess / $($KEYS.Count)" -ForegroundColor Green

# Step 5 - verify each key has exactly 3 replicas
Write-Host "`n[5] Verifying 3 replicas per key..." -ForegroundColor Yellow
$replicaOk = 0
$replicaWarn = 0

foreach ($k in $KEYS) {
    $count = 0
    foreach ($port in $PORTS) {
        try {
            Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null
            $count++
        } catch {}
    }
    if ($count -eq 3) { $replicaOk++ }
    else {
        Write-Host "    WARN: $k has $count copies (expected 3)" -ForegroundColor Yellow
        $replicaWarn++
    }
}

Write-Host "    Keys with exactly 3 replicas: $replicaOk / $($KEYS.Count)" -ForegroundColor Green
if ($replicaWarn -gt 0) {
    Write-Host "    Keys with wrong replica count: $replicaWarn" -ForegroundColor Yellow
}

# Summary
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host " Test Summary" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host " Puts succeeded   : $putSuccess / $($KEYS.Count)"
Write-Host " Gets correct     : $getSuccess / $($KEYS.Count)"
Write-Host " 3-replica keys   : $replicaOk / $($KEYS.Count)"
if ($putFailed -eq 0 -and $getFailed -eq 0 -and $replicaWarn -eq 0) {
    Write-Host "`n ALL TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "`n SOME TESTS FAILED - check output above" -ForegroundColor Red
}
