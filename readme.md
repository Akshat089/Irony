# IronRing

A distributed key-value store built in Rust. IronRing uses consistent hashing with virtual nodes to partition keys across worker nodes, quorum-based replication to ensure durability, and heartbeat-based failure detection with automatic re-replication to recover from node failures.

The name reflects the two core ideas — Rust (iron) and the consistent hash ring (ring).

---

## Architecture

```
                        ┌─────────────────────┐
                        │     Controller       │
                        │                      │
                        │  - Hash ring state   │
                        │  - Node registry     │
                        │  - Failure detection │
                        │  - Re-replication    │
                        └──────────┬───────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │ register + heartbeat every 2s            │
              │                    │                     │
    ┌─────────▼──────┐  ┌──────────▼─────┐  ┌──────────▼─────┐  ┌─────────────────┐
    │    Worker 1    │  │    Worker 2    │  │    Worker 3    │  │    Worker 4    │
    │                │  │                │  │                │  │                │
    │  - DashMap KV  │  │  - DashMap KV  │  │  - DashMap KV  │  │  - DashMap KV  │
    │  - PUT / GET   │  │  - PUT / GET   │  │  - PUT / GET   │  │  - PUT / GET   │
    │  - Replication │  │  - Replication │  │  - Replication │  │  - Replication │
    └────────────────┘  └────────────────┘  └────────────────┘  └────────────────┘

Client Flow:
  1. Client queries GET /v1/ring from controller to get ring state
  2. Client hashes key and identifies primary worker
  3. Client sends PUT/GET directly to primary worker
  4. Primary replicates to 2 replica workers (quorum = 2 of 3)
```

---

## How It Works

### Consistent Hashing with Virtual Nodes

Keys are distributed across workers using a consistent hash ring. Each physical worker is placed at 150 positions (virtual nodes) on a ring spanning 0 to 2^64. When a key arrives, it is hashed using SHA-256 and the first worker clockwise from that hash position becomes the primary. Adding or removing a worker only remaps that worker's keys — all other keys stay in place. This is the same approach used by Amazon DynamoDB and Apache Cassandra.

### Quorum Writes

Every key is stored on 3 workers — a primary and 2 replicas. When a PUT arrives at the primary it writes locally and synchronously replicates to the first replica. Once both confirm, it returns success to the client. The third replica is written asynchronously in the background. This means the system requires 2 of 3 confirmations (quorum) before acknowledging a write — providing durability without waiting for all 3 replicas sequentially.

### Heartbeat Failure Detection

Every worker sends a heartbeat POST to the controller every 2 seconds. The controller runs a background task that checks timestamps every 5 seconds. A worker that has not heartbeated in 6 seconds is marked Suspect. After 12 seconds it is marked Dead and removed from the ring. The ring version increments on every topology change and surviving workers detect this via their next heartbeat response and refresh their local ring copy.

### Automatic Re-replication

When a worker is marked Dead, affected keys drop from 3 copies to 2. The controller identifies which keys are under-replicated by querying all surviving workers, determines which node in the new replica set is missing the key, and instructs a surviving holder to push the key to the new target. After re-replication completes every key is back to 3 copies.

---

## CAP Theorem Position

IronRing is an **AP system** — it prioritizes Availability and Partition Tolerance over strict Consistency. The asynchronous third replica write means there is a brief window where replica 3 may not have the latest value. This is the same tradeoff made by DynamoDB and Cassandra and is the correct choice for a key-value store that must remain responsive under failure.

---

## API Reference

### Controller — `http://localhost:9090`

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/ring` | Returns the full ring state — nodes, virtual node positions, ring version |
| GET | `/v1/nodes` | Returns all registered nodes with current health status |
| GET | `/v1/health` | Liveness check — always returns 200 if controller is running |
| GET | `/v1/metrics` | Controller runtime metrics |
| POST | `/v1/nodes/register` | Workers call this on startup to join the cluster |
| POST | `/v1/heartbeat` | Workers call this every 2 seconds to signal they are alive |
| GET | `/v1/metrics` | Returns the total nodes, how many dead,alive,suspect, also gives us the runtime of controller |
| GET | `/v1/status` | This endpoint shows the result of the last re-replication event such as last replication node and the time it happened at, which keys were replicated and which weren't.|

### Worker — `http://localhost:808{1-4}`

| Method | Path | Description |
|--------|------|-------------|
| GET | `/v1/keys/{key}` | Read a value by key |
| PUT | `/v1/keys/{key}` | Write a key-value pair with quorum replication |
| GET | `/v1/keys` | Dump all key-value pairs stored on this worker |
| POST | `/v1/replicate` | Internal — primary calls this to replicate to a replica |
| POST | `/v1/replicate-to` | Internal — controller calls this during re-replication |
| GET | `/v1/health` | Liveness check |
| GET | `/v1/metrics` | Worker runtime metrics |

### Example Requests

**Write a key:**
```bash
curl -X PUT http://localhost:8081/v1/keys/username \
  -H "Content-Type: application/json" \
  -d '{"value":"alice"}'
```
```json
{"key":"username","success":true,"replicas_confirmed":2}
```

**Read a key:**
```bash
curl http://localhost:8081/v1/keys/username
```
```json
{"key":"username","value":"alice","served_by":"worker-01"}
```

**Get ring state:**
```bash
curl http://localhost:9090/v1/ring
```
```json
{
  "nodes": [...],
  "virtual_nodes": [...],
  "replication_factor": 3,
  "ring_version": 4
}
```

**Get controller metrics:**
```bash
curl http://localhost:9090/v1/metrics
```
```json
{
  "total_nodes": 4,
  "alive_nodes": 4,
  "suspect_nodes": 0,
  "dead_nodes": 0,
  "ring_version": 4,
  "re_replication_count": 0,
  "uptime_seconds": 120
}
```

---

## Running IronRing

### Prerequisites

- Rust 1.76+ with Cargo
- Docker and Docker Compose

### With Docker Compose (Recommended)

```bash
# Start all 5 nodes
make run

# Or without make
docker compose up --build
```

All 5 containers start automatically. Workers wait for the controller health check before registering. Once up, the controller is available at `http://localhost:9090` and workers at ports 8081-8084.

```bash
# Stop everything
make stop

# Simulate worker failure
make kill-worker

# Bring worker back
make revive-worker

# View logs
make logs
```

### Local Development (Without Docker)

Start the controller in one terminal:
```bash
HOST=127.0.0.1 NODE_PORT=9090 cargo run -p controller
```

Start workers in separate terminals:
```bash
NODE_ID=worker-1 NODE_PORT=8081 HOST=127.0.0.1 CONTROLLER_ADDR=http://127.0.0.1:9090 cargo run -p worker
NODE_ID=worker-2 NODE_PORT=8082 HOST=127.0.0.1 CONTROLLER_ADDR=http://127.0.0.1:9090 cargo run -p worker
NODE_ID=worker-3 NODE_PORT=8083 HOST=127.0.0.1 CONTROLLER_ADDR=http://127.0.0.1:9090 cargo run -p worker
NODE_ID=worker-4 NODE_PORT=8084 HOST=127.0.0.1 CONTROLLER_ADDR=http://127.0.0.1:9090 cargo run -p worker
```

### Running Tests

```bash
# Basic read/write and replication test
.\scripts\test_basic.ps1

# Failure detection and re-replication test
.\scripts\test_failure.ps1

# Worker recovery test
.\scripts\test_recovery.ps1

# Full end-to-end demo
.\scripts\demo.ps1
```

---

## Project Structure

```
ironring/
├── Cargo.toml              # Workspace root
├── Dockerfile              # Two-stage build for both binaries
├── docker-compose.yml      # 1 controller + 4 workers
├── Makefile                # Common commands
├── README.md
├── scripts/
│   ├── test_basic.ps1      # PUT/GET/replication verification
│   ├── test_failure.ps1    # Failure detection + re-replication test
│   ├── test_recovery.ps1   # Worker recovery test
│   └── demo.ps1            # Full end-to-end demo
├── common/                 # Shared library crate
│   └── src/
│       ├── lib.rs
│       ├── models.rs       # All shared request/response structs
│       ├── ring.rs         # Consistent hash ring implementation
│       └── errors.rs       # Shared error types
├── controller/             # Controller binary
│   └── src/
│       ├── main.rs         # Startup, router, background task spawns
│       ├── state.rs        # Shared application state
│       ├── routes.rs       # HTTP route handlers
│       ├── heartbeat.rs    # Failure detection background task
│       └── replication.rs  # Re-replication logic
└── worker/                 # Worker binary
    └── src/
        ├── main.rs         # Startup, registration, router
        ├── state.rs        # Shared application state + DashMap
        ├── routes.rs       # HTTP route handlers
        ├── heartbeat.rs    # Heartbeat sender background task
        └── replication.rs  # Quorum write logic
```

---

## Tech Stack

| Crate | Purpose |
|-------|---------|
| `axum` | HTTP server framework for both controller and worker REST APIs |
| `tokio` | Async runtime — powers concurrent request handling and background tasks |
| `reqwest` | HTTP client for worker-to-worker replication and controller-to-worker calls |
| `dashmap` | Concurrent HashMap for thread-safe in-memory key-value storage on workers |
| `serde` + `serde_json` | Serialization and deserialization of all request and response types |
| `sha2` | SHA-256 hashing for placing keys and virtual nodes on the consistent hash ring |
| `uuid` | Generating unique node identifiers on startup |
| `chrono` | Timestamps for heartbeat tracking and uptime calculation |
| `tracing` | Structured logging across all nodes |
| `tracing-subscriber` | Log output formatting and filtering via RUST_LOG |
| `dotenv` | Loading environment variables from .env for local development |

---

## Known Limitations

**Recovered workers start empty.** When a dead worker restarts it re-registers with the controller and rejoins the ring, but its DashMap is empty. It does not automatically re-sync the keys it previously held. New writes will route to it correctly going forward. Full data restoration on recovery would require a gossip protocol or a dedicated sync endpoint and is outside the current scope.

**In-memory storage only.** All data lives in DashMap and is lost when a container restarts. Adding persistent storage (RocksDB, sled) would make this production-grade.

**Single controller.** The controller is a single point of failure. A production system would use Raft or Paxos to replicate controller state across multiple nodes. This is intentionally out of scope for this implementation.

---

## Design Inspiration

IronRing implements the same core distributed systems concepts as:

- **Amazon DynamoDB** — consistent hashing, virtual nodes, quorum writes, AP consistency model
- **Apache Cassandra** — gossip-based failure detection (simplified here to heartbeats), tunable consistency
- **Amazon S3** — background replication for durability without sacrificing write latency