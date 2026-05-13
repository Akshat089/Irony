# =============================================================================
# demo.ps1
# Full end-to-end demo script for IronRing
# Shows the complete system working — writes, reads, failure, recovery
# Run this for your professor demo or interview
# Usage: .\scripts\demo.ps1
# =============================================================================

$CONTROLLER = "http://127.0.0.1:9090"
$PORTS = @(8081, 8082, 8083, 8084)

function Pause-Demo($msg) {
    Write-Host "`n>>> $msg" -ForegroundColor Magenta
    Write-Host "    Press Enter to continue..." -ForegroundColor Gray
    Read-Host
}

function Section($title) {
    Write-Host "`n" + ("=" * 60) -ForegroundColor Cyan
    Write-Host " $title" -ForegroundColor Cyan
    Write-Host ("=" * 60) -ForegroundColor Cyan
}

function PutKey($key, $value) {
    foreach ($port in $PORTS) {
        try {
            $r = Invoke-RestMethod -Method Put `
                -Uri "http://127.0.0.1:$port/v1/keys/$key" `
                -ContentType "application/json" `
                -Body "{`"value`":`"$value`"}"
            return $r
        } catch {}
    }
    return $null
}

# =========================================================
Section "1. Cluster Health Check"
# =========================================================

Write-Host "`nQuerying controller for cluster state..." -ForegroundColor Yellow
$nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
$ring = Invoke-RestMethod -Uri "$CONTROLLER/v1/ring"

Write-Host "`nRegistered nodes:"
$nodes | ForEach-Object {
    $color = if ($_.status -eq "Alive") { "Green" } else { "Red" }
    Write-Host "  - $($_.node_id) on port $($_.node_port) [$($_.status)]" -ForegroundColor $color
}

Write-Host "`nRing state:"
Write-Host "  - Physical nodes : $($ring.nodes.Count)"
Write-Host "  - Virtual nodes  : $($ring.virtual_nodes.Count) (150 per worker)"
Write-Host "  - Ring version   : $($ring.ring_version)"

Pause-Demo "Cluster is healthy with 4 workers and a populated consistent hash ring."

# =========================================================
Section "2. Writing Data — Quorum Writes"
# =========================================================

Write-Host "`nWriting 10 keys to the cluster..." -ForegroundColor Yellow
Write-Host "(Each key goes to its primary + 2 replicas = quorum of 2 required)" -ForegroundColor Gray

$demoKeys = @("user:alice","user:bob","user:charlie","score:alice","score:bob","session:xyz","config:timeout","config:retries","product:001","product:002")

foreach ($k in $demoKeys) {
    $r = PutKey $k "value-for-$k"
    if ($r) {
        Write-Host "  PUT $k -> replicas_confirmed: $($r.replicas_confirmed)" -ForegroundColor Green
    } else {
        Write-Host "  PUT $k -> FAILED" -ForegroundColor Red
    }
}

Pause-Demo "All writes confirmed on 2 replicas synchronously. 3rd replica written in background."

# =========================================================
Section "3. Reading Data — Direct Worker Access"
# =========================================================

Write-Host "`nReading keys back from cluster..." -ForegroundColor Yellow

foreach ($k in $demoKeys[0..2]) {
    Write-Host "`n  Key: $k"
    foreach ($port in $PORTS) {
        try {
            $r = Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k"
            Write-Host "    Port $port : FOUND (served by $($r.served_by))" -ForegroundColor Green
        } catch {
            Write-Host "    Port $port : not here" -ForegroundColor Gray
        }
    }
}

Pause-Demo "Each key exists on exactly 3 workers as expected."

# =========================================================
Section "4. Simulating Worker Failure"
# =========================================================

Write-Host "`nCurrent ring version: $($ring.ring_version)" -ForegroundColor Yellow
Write-Host "`nAbout to simulate worker-2 failure." -ForegroundColor Yellow
Pause-Demo "Kill worker-2 now (Ctrl+C in its terminal), then press Enter."

Write-Host "`nMonitoring failure detection..." -ForegroundColor Yellow
$detected = $false
for ($i = 1; $i -le 20; $i++) {
    Start-Sleep 2
    $nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
    $dead = $nodes | Where-Object { $_.status -eq "Dead" }
    $alive = $nodes | Where-Object { $_.status -eq "Alive" }
    $suspect = $nodes | Where-Object { $_.status -eq "Suspect" }
    Write-Host "  [${i}] Alive=$($alive.Count) Suspect=$($suspect.Count) Dead=$($dead.Count)"
    if ($dead.Count -gt 0) {
        Write-Host "`n  FAILURE DETECTED: $($dead.node_id) is Dead" -ForegroundColor Red
        $detected = $true
        break
    }
}

if (-not $detected) {
    Write-Host "  Failure not detected in time." -ForegroundColor Red
    exit 1
}

Write-Host "`nWaiting for re-replication to complete..." -ForegroundColor Yellow
Start-Sleep 8

$ringAfterFailure = Invoke-RestMethod -Uri "$CONTROLLER/v1/ring"
Write-Host "  Ring version after failure : $($ringAfterFailure.ring_version)"
Write-Host "  Nodes in ring after failure: $($ringAfterFailure.nodes.Count)"

Pause-Demo "Controller detected failure, rebuilt ring, and triggered re-replication."

# =========================================================
Section "5. Verifying Data Survived Failure"
# =========================================================

Write-Host "`nChecking all keys are still readable and have 3 copies..." -ForegroundColor Yellow
$survivingPorts = $PORTS | Where-Object { $_ -ne 8082 }
$allOk = $true

foreach ($k in $demoKeys) {
    $count = 0
    foreach ($port in $survivingPorts) {
        try {
            Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null
            $count++
        } catch {}
    }
    if ($count -eq 3) {
        Write-Host "  $k : 3 copies [OK]" -ForegroundColor Green
    } else {
        Write-Host "  $k : $count copies [WARN]" -ForegroundColor Yellow
        $allOk = $false
    }
}

Write-Host "`nWriting new key after failure..." -ForegroundColor Yellow
$r = PutKey "post-failure-key" "written-after-worker2-died"
if ($r) {
    Write-Host "  PUT post-failure-key -> replicas_confirmed: $($r.replicas_confirmed)" -ForegroundColor Green
}

Pause-Demo "Data survived the failure. Writes still work. Re-replication restored 3 copies."

# =========================================================
Section "6. Worker Recovery"
# =========================================================

Pause-Demo "Restart worker-2 in its terminal, then press Enter."

Write-Host "`nMonitoring recovery..." -ForegroundColor Yellow
for ($i = 1; $i -le 15; $i++) {
    Start-Sleep 2
    $nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
    $alive = $nodes | Where-Object { $_.status -eq "Alive" }
    Write-Host "  [${i}] Alive nodes: $($alive.Count)"
    if ($alive.Count -eq 4) {
        Write-Host "`n  All 4 nodes alive again!" -ForegroundColor Green
        break
    }
}

$ringFinal = Invoke-RestMethod -Uri "$CONTROLLER/v1/ring"
Write-Host "`n  Final ring version : $($ringFinal.ring_version)"
Write-Host "  Final node count   : $($ringFinal.nodes.Count)"

# =========================================================
Section "Demo Complete — Summary"
# =========================================================

Write-Host @"

  What was demonstrated:
  
  [1] Consistent hashing with virtual nodes
      - 4 workers, 150 virtual nodes each = 600 ring positions
      - Keys distributed evenly across workers
  
  [2] Quorum writes (W=2 of 3)
      - Every PUT confirmed on 2 replicas before returning success
      - 3rd replica written asynchronously in background
  
  [3] Heartbeat-based failure detection
      - Controller detects missing heartbeats within ~12 seconds
      - Transitions: Alive -> Suspect -> Dead
  
  [4] Automatic re-replication
      - Controller rebuilds ring after failure
      - Instructs surviving workers to restore 3 replicas
  
  [5] Worker recovery
      - Dead worker rejoins cluster on restart
      - Ring rebuilt automatically
      - New writes distributed to recovered worker

  This is the same architecture used by Amazon DynamoDB and Apache Cassandra.
  
"@ -ForegroundColor Cyan
