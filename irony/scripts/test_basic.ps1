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
Write-Host " IronRing Basic Test" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

Write-Host "`n[1] Checking cluster health..." -ForegroundColor Yellow
$nodes = Invoke-RestMethod -Uri "$CONTROLLER/v1/nodes"
$aliveCount = ($nodes | Where-Object { $_.status -eq "Alive" }).Count
Write-Host "    Alive nodes: $aliveCount / $($nodes.Count)"
if ($aliveCount -lt 4) {
    Write-Host "    ERROR: Not all nodes alive." -ForegroundColor Red
    exit 1
}
Write-Host "    All nodes alive." -ForegroundColor Green

Write-Host "`n[2] Checking ring state..." -ForegroundColor Yellow
$ring = Invoke-RestMethod -Uri "$CONTROLLER/v1/ring"
Write-Host "    Nodes in ring : $($ring.nodes.Count)"
Write-Host "    Virtual nodes : $($ring.virtual_nodes.Count)"
if ($ring.nodes.Count -lt 4) {
    Write-Host "    ERROR: Ring incomplete." -ForegroundColor Red
    exit 1
}
Write-Host "    Ring healthy." -ForegroundColor Green

Write-Host "`n[3] Writing 20 keys..." -ForegroundColor Yellow
$putSuccess = 0
$putFailed = 0
foreach ($k in $KEYS) {
    $r = PutKey $k "value-$k"
    if ($r) { $putSuccess++ }
    else {
        Write-Host "    FAILED: $k" -ForegroundColor Red
        $putFailed++
    }
}
Write-Host "    Written: $putSuccess / $($KEYS.Count)" -ForegroundColor Green

Write-Host "`n[4] Reading all keys back..." -ForegroundColor Yellow
$getSuccess = 0
foreach ($k in $KEYS) {
    foreach ($port in $PORTS) {
        try {
            $r = Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k"
            if ($r.value -eq "value-$k") { $getSuccess++; break }
        } catch {}
    }
}
Write-Host "    Correct values: $getSuccess / $($KEYS.Count)" -ForegroundColor Green

Write-Host "`n[5] Verifying 3 replicas per key..." -ForegroundColor Yellow
$replicaOk = 0
$replicaWarn = 0
foreach ($k in $KEYS) {
    $count = 0
    foreach ($port in $PORTS) {
        try { Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/keys/$k" | Out-Null; $count++ } catch {}
    }
    if ($count -eq 3) { $replicaOk++ }
    else { Write-Host "    WARN: $k has $count copies" -ForegroundColor Yellow; $replicaWarn++ }
}
Write-Host "    3-replica keys: $replicaOk / $($KEYS.Count)" -ForegroundColor Green

Write-Host "`n============================================" -ForegroundColor Cyan
Write-Host " Test Summary" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host " Puts succeeded : $putSuccess / $($KEYS.Count)"
Write-Host " Gets correct   : $getSuccess / $($KEYS.Count)"
Write-Host " 3-replica keys : $replicaOk / $($KEYS.Count)"
if ($putFailed -eq 0 -and $replicaWarn -eq 0 -and $getSuccess -eq $KEYS.Count) {
    Write-Host "`n ALL TESTS PASSED" -ForegroundColor Green
} else {
    Write-Host "`n SOME TESTS FAILED" -ForegroundColor Red
}