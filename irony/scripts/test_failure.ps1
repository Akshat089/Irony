$CONTROLLER = "http://127.0.0.1:9090"
$PORTS = @(8081, 8082, 8083, 8084)
$KEYS = @("alpha","bravo","charlie","delta","echo","foxtrot","golf","hotel","india","juliet","kilo","lima","mike","november","oscar","papa","quebec","romeo","sierra","tango")

function PutKey($key, $value) {
    foreach ($port in $PORTS) {
        try {
            return Invoke-RestMethod -Method Put `
                -Uri "http://127.0.0.1:$port/v1/keys/$key" `
                -ContentType "application/json" `
                -Body "{`"value`":`"$value`"}"
        } catch {
            $body = $_.ErrorDetails.Message | ConvertFrom-Json -ErrorAction SilentlyContinue
            if ($body.redirect_to) {
                $redirectPort = ([System.Uri]$body.redirect_to).Port
                try {
                    return Invoke-RestMethod -Method Put `
                        -Uri "http://127.0.0.1:$redirectPort/v1/keys/$key" `
                        -ContentType "application/json" `
                        -Body "{`"value`":`"$value`"}"
                } catch {}
            }
        }
    }
    return $null
}

Write-Host "============================================" -ForegroundColor Cyan
Write-Host " IronRing Failure and Re-replication Test" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

# Step 1 - snapshot replica counts before failure
Write-Host "`n[1] Snapshotting replica counts before failure..." -ForegroundColor Yellow
$before = @{}
foreach ($k in $KEYS) {
    $count = 0
    foreach ($port in $PORTS) {
        try { Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null; $count++ } catch {}
    }
    $before[$k] = $count
}
$allThree = ($before.Values | Where-Object { $_ -eq 3 }).Count
Write-Host "    Keys with 3 replicas before failure: $allThree / $($KEYS.Count)" -ForegroundColor Green

# Step 2 - kill worker2
Write-Host "`n[2] Stopping worker2 container..." -ForegroundColor Yellow
docker compose stop worker2
Write-Host "    worker2 stopped." -ForegroundColor Red

# Step 3 - monitor until dead
Write-Host "`n[3] Monitoring failure detection..." -ForegroundColor Yellow
$detected = $false
for ($i = 1; $i -le 20; $i++) {
    Start-Sleep 2
    $nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
    $dead = $nodes | Where-Object { $_.status -eq "Dead" }
    $suspect = $nodes | Where-Object { $_.status -eq "Suspect" }
    $alive = $nodes | Where-Object { $_.status -eq "Alive" }
    Write-Host "    [$i] Alive=$($alive.Count) Suspect=$($suspect.Count) Dead=$($dead.Count)"
    if ($dead.Count -gt 0) {
        Write-Host "    Failure detected: $($dead.node_id)" -ForegroundColor Red
        $detected = $true
        break
    }
}

if (-not $detected) {
    Write-Host "    Failure not detected in time." -ForegroundColor Red
    exit 1
}

# Step 4 - wait for re-replication
Write-Host "`n[4] Waiting 10s for re-replication..." -ForegroundColor Yellow
Start-Sleep 10

# Step 5 - verify 3 copies on surviving workers
Write-Host "`n[5] Verifying replica counts after re-replication..." -ForegroundColor Yellow
$survivingPorts = @(8081, 8083, 8084)
$replicaOk = 0
$replicaWarn = 0
foreach ($k in $KEYS) {
    $count = 0
    foreach ($port in $survivingPorts) {
        try { Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null; $count++ } catch {}
    }
    if ($count -eq 3) { $replicaOk++ }
    else {
        Write-Host "    WARN: $k has $count copies on surviving workers" -ForegroundColor Yellow
        $replicaWarn++
    }
}
Write-Host "    Keys with 3 copies: $replicaOk / $($KEYS.Count)" -ForegroundColor Green

# Step 6 - verify reads still work
Write-Host "`n[6] Verifying reads after failure..." -ForegroundColor Yellow
$readOk = 0
foreach ($k in $KEYS) {
    foreach ($port in $survivingPorts) {
        try { Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null; $readOk++; break } catch {}
    }
}
Write-Host "    Readable keys: $readOk / $($KEYS.Count)" -ForegroundColor Green

# Step 7 - verify writes still work
Write-Host "`n[7] Verifying writes after failure..." -ForegroundColor Yellow
$r = PutKey "post-failure-key" "written-after-failure"
if ($r) {
    Write-Host "    Write succeeded. Replicas confirmed: $($r.replicas_confirmed)" -ForegroundColor Green
} else {
    Write-Host "    Write FAILED" -ForegroundColor Red
}

# Step 8 - revive worker2
Write-Host "`n[8] Restarting worker2..." -ForegroundColor Yellow
docker compose start worker2
Start-Sleep 5
$nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
$alive = $nodes | Where-Object { $_.status -eq "Alive" }
Write-Host "    Alive nodes after recovery: $($alive.Count)" -ForegroundColor Green

# Summary
Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host " Failure Test Summary" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host " Failure detected        : $(if ($detected) { 'YES' } else { 'NO' })"
Write-Host " Keys re-replicated (3x) : $replicaOk / $($KEYS.Count)"
Write-Host " Reads working           : $readOk / $($KEYS.Count)"
Write-Host " Writes working          : $(if ($r) { 'YES' } else { 'NO' })"
Write-Host " Nodes alive at end      : $($alive.Count) / 4"

if ($detected -and $replicaOk -eq $KEYS.Count -and $readOk -eq $KEYS.Count -and $r) {
    Write-Host "`n ALL FAILURE TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "`n SOME TESTS FAILED - check output above" -ForegroundColor Red
}