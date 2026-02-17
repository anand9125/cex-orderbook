# ⚡ Perp CEX — High-Performance Perpetual Futures Exchange

A production-grade, ultra-low latency centralized exchange (CEX) for perpetual futures, built in Rust.

> 🚧 **Status: Active Development** — Matching engine and orderbook complete. Event pipeline, position engine, and wallet engine in progress.

---

## 🎯 What Is This?

A from-scratch perpetual futures exchange engine designed around the same architecture principles used by production exchanges like Binance and Bybit.

The core philosophy is simple:

- **Matching engine does one thing** — match orders, nothing else
- **No I/O in the hot path** — no database, no network calls during matching
- **Events flow outward** — everything downstream reads from a ring buffer
- **Deterministic by design** — single-threaded matching means reproducible state

---

## ✅ What's Built So Far

### 1. Matching Engine (Core 0)
- Single-threaded, CPU-pinned (Core 0) order matching
- Price-time priority (FIFO at each price level)
- Batch processing (up to 256 orders per batch)
- Priority queue — liquidations → cancels → market → limit
- Handles `Limit` and `Market` order types
- Validates leverage (1–125x), quantity, price
- Generates `Fill` events on every match
- Emits `OrderPlaced`, `OrderCancelled`, `OrderRejected` events
- Real-time OS scheduling (`SCHED_FIFO`, priority 99)

### 2. Orderbook
- In-memory `BTreeMap<Price, VecDeque<Order>>` per side
- Bids sorted descending (highest first)
- Asks sorted ascending (lowest first)
- `HashMap<OrderId, Order>` for O(1) order lookup
- `HashMap<UserId, HashSet<OrderId>>` for fast user order index
- Best bid/ask, spread calculation
- Partial fill support

### 3. Ring Buffer (Lockless SPSC)
- Single-producer (matching engine), single-consumer (event pipeline)
- Lock-free using atomic operations only
- Power-of-2 capacity for fast modulo via bitwise AND
- Cache-line aligned to prevent false sharing
- `push()` — non-blocking write (~10ns)
- `try_pop()` / `drain_batch()` — non-blocking reads
- 1M event capacity (~16 MB RAM, ~25 sec buffer at 40k orders/sec)

### 4. HTTP API (Axum)
- `POST /signup` — Register user
- `POST /signin` — Authenticate user
- `POST /place_order` — Submit order to matching engine
- `POST /cancel` — Cancel open order
- Shared `AppState` via `Arc` (thread-safe)
- `std::sync::mpsc` channels for HTTP → Engine communication

### 5. Database (PostgreSQL)
- User account storage
- Connected via `sqlx`

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     CLIENT (HTTP)                        │
└────────────────────────┬────────────────────────────────┘
                         │ POST /place_order
                         ▼
┌─────────────────────────────────────────────────────────┐
│               AXUM HTTP SERVER                           │
│  • Auth, validation                                     │
│  • Deserialize JSON → Order                             │
│  • Send via mpsc::sync_channel → Engine                 │
│  • Wait for reply via mpsc::channel (2s timeout)        │
└────────────────────────┬────────────────────────────────┘
                         │ std::sync::mpsc (10k capacity)
                         ▼
┌─────────────────────────────────────────────────────────┐
│         MATCHING ENGINE  (Core 0 — isolated)            │
│                                                         │
│  1. recv()      → blocking wait for first command      │
│  2. try_recv()  → drain up to 256 more (non-blocking)  │
│  3. sort_by_key → liquidations first (priority 0→3)    │
│  4. match_order → walk orderbook, generate fills       │
│  5. push()      → write events to ring buffer          │
│  6. send()      → reply to HTTP layer via mpsc         │
│                                                         │
│  ORDERBOOK:                                             │
│  BTreeMap<Price, VecDeque<Order>>  (bids + asks)       │
│  HashMap<OrderId, Order>           (fast lookup)       │
│  HashMap<UserId, HashSet<OrderId>> (user index)        │
└────────────────────────┬────────────────────────────────┘
                         │ Ring Buffer (1M events, lockless)
                         ▼
┌─────────────────────────────────────────────────────────┐
│         EVENT PIPELINE  (Core 2) — 🚧 Coming Soon       │
│                                                         │
│  ┌──────────────────┐   ┌───────────────────────────┐  │
│  │  Kafka Producer  │   │  WebSocket Broadcaster    │  │
│  │  (durable WAL)   │   │  (real-time updates)      │  │
│  └──────────────────┘   └───────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│         POSITION ENGINE  (Core 1) — 🚧 Coming Soon      │
│  • PnL calculation                                      │
│  • Margin ratio tracking                                │
│  • Liquidation detection                                │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│         WALLET ENGINE   (Core 1) — 🚧 Coming Soon       │
│  • Reserve / release margin                             │
│  • Balance settlement                                   │
│  • Double-entry accounting                              │
└─────────────────────────────────────────────────────────┘
```

---

## 🧵 Thread Layout

| Thread | Core | Type | Responsibility |
|--------|------|------|----------------|
| `matching-engine` | 0 (isolated) | `std::thread` | Pure order matching |
| `event-pipeline` | 2 | Tokio task | Kafka + WebSocket |
| `axum-worker-1` | 3 | Tokio task | HTTP handling |
| `axum-worker-2` | 3 | Tokio task | HTTP handling |

> Core 0 is isolated at the OS level (`isolcpus=0`) so the kernel never schedules other processes there. The matching engine runs uninterrupted.

---

## ⚡ Performance

| Component | Throughput | Latency (P99) |
|-----------|------------|---------------|
| Matching Engine | 40,000 orders/sec | < 100μs |
| Ring Buffer write | 100M events/sec | ~10ns |
| Ring Buffer read (batch) | 50M events/sec | ~15ns |
| HTTP → Engine → Reply | — | < 5ms |

---

## 🗂️ Project Structure

```
perp-cex/
├── src/
│   ├── main.rs            # Entry point, thread setup
│   ├── engine.rs          # Matching engine + event loop
│   ├── types.rs           # Order, Fill, Event, OrderBookMessage
│   ├── models.rs          # Request/response structs
│   ├── state.rs           # AppState (shared across handlers)
│   ├── auth.rs            # create_user, signin handlers
│   └── orderbook/
│       ├── mod.rs         # OrderBook struct
│       ├── matching.rs    # match_order, insert_order, cancel_order
│       └── ring_buffer.rs # Lockless SPSC ring buffer
├── db/
│   ├── src/
│   │   └── lib.rs         # Db struct, connection pool
│   └── Cargo.toml
└── Cargo.toml
```

---

## 🚀 Getting Started

### Prerequisites

- Rust (stable, 1.75+)
- PostgreSQL
- Linux (recommended for CPU pinning + real-time priority)

### 1. Clone & Configure

```bash
git clone https://github.com/yourname/perp-cex
cd perp-cex
cp .env.example .env
# Edit .env with your DATABASE_URL
```

### 2. Set Up Database

```bash
createdb perp_cex
sqlx migrate run
```

### 3. Build & Run

```bash
# Development
cargo run

# Production (optimized)
cargo build --release
./target/release/perp-cex
```

### 4. Linux: CPU Isolation (Recommended for Production)

```bash
# Isolate Core 0 for matching engine
sudo nano /etc/default/grub
# Add to GRUB_CMDLINE_LINUX: isolcpus=0 nohz_full=0 rcu_nocbs=0
sudo update-grub && sudo reboot

# Grant real-time scheduling without sudo
sudo setcap cap_sys_nice=eip ./target/release/perp-cex
```

---

## 📡 API Reference

### `POST /signup`
```json
// Request
{ "email": "user@example.com", "password": "secret" }

// Response
{ "user_id": 1, "email": "user@example.com" }
```

### `POST /signin`
```json
// Request
{ "email": "user@example.com", "password": "secret" }

// Response
{ "token": "jwt_token_here", "user_id": 1 }
```

### `POST /place_order`
```json
// Request
{
  "user_id": 1,
  "symbol": "BTC-PERP",
  "side": "Buy",
  "order_type": "Limit",
  "price": "50000.00",
  "quantity": "1.5",
  "leverage": "10"
}

// Response
{
  "order_id": 42,
  "status": "New",
  "filled": "0",
  "remaining": "1.5"
}
```

### `POST /cancel`
```json
// Request
{ "order_id": 42, "user_id": 1 }

// Response
{
  "order_id": 42,
  "status": "Cancelled",
  "filled": "0.5",
  "remaining": "1.0"
}
```

---

## 🛣️ Roadmap

- [x] Orderbook (BTreeMap, price-time priority)
- [x] Matching engine (single-threaded, CPU-pinned)
- [x] Ring buffer (lockless SPSC)
- [x] HTTP API (Axum, signup/signin/place_order/cancel)
- [x] PostgreSQL (user storage)
- [ ] Event pipeline (Kafka producer + WebSocket broadcaster)
- [ ] Position engine (PnL, margin ratio, liquidation detection)
- [ ] Wallet engine (reserve/release margin, balance settlement)
- [ ] Oracle integration (mark price from Binance/Bybit)
- [ ] Crash recovery (rebuild orderbook from Kafka WAL)
- [ ] Redis hot state (positions + balances)
- [ ] Prometheus metrics + Grafana dashboards
- [ ] Stop orders (StopMarket, StopLimit)
- [ ] Multi-symbol support (ETH-PERP, SOL-PERP, ...)

---

## 🔧 Tech Stack

| Layer | Technology |
|-------|------------|
| Language | Rust |
| HTTP Server | Axum |
| Async Runtime | Tokio |
| Database | PostgreSQL + sqlx |
| Serialization | serde + serde_json |
| Decimal Math | rust_decimal |
| Config | dotenvy |
| Event Bus | Ring Buffer (custom, lockless) |
| Future: Messaging | Apache Kafka |
| Future: Cache | Redis |

---

## ⚠️ Disclaimer

This project is for educational and research purposes. It is **not production-ready**. Running a real exchange requires regulatory compliance, security audits, and significantly more infrastructure than what is demonstrated here.

---

## 📄 License

MIT
