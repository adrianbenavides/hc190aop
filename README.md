## Overview

This project implements a transaction processing engine that handles deposits, withdrawals, disputes, resolutions, and
chargebacks from CSV input and outputs the final state of client accounts.

### Key Features

- **Sequential Consistency:** Transactions are processed in strict chronological order as they are read from the CSV
  stream.
- **Data Integrity:** Accurate decimal arithmetic using `rust_decimal` (supporting up to 4 decimal places as required).
- **Fault Tolerant:** Robust error handling with `miette` and graceful skipping of invalid rows.
- **Scalable Storage:** Pluggable backends supporting In-Memory and persistent RocksDB for large datasets.

## Project Structure

The codebase is organized following Domain-Driven Design (DDD) principles:

- **Domain:** Core business logic and entities (`ClientAccount`, `Transaction`).
- **Application:** Orchestration and engine logic (`PaymentEngine`).
- **Infrastructure:** Persistence implementations (`InMemory`, `RocksDB`).
- **Interfaces:** Input/Output handlers (CSV reader/writer).

## Installation

```bash
cargo build --release
```

## Usage

```bash
cargo run -- transactions.csv > accounts.csv
```

## Correctness & Testing

### Testing Strategy

- **Unit Tests:** Extensive unit tests for domain logic (`ClientAccount`, `Balance`) and individual components.
- **Integration Tests:** End-to-end CLI tests using `assert_cmd` and `fixtures` to verify the complete transaction
  lifecycle.
- **Property-Based Testing:** Utilizing `proptest` to validate transaction logic against a wide range of generated
  inputs, ensuring robustness in complex edge cases (e.g., specific sequences of disputes and resolutions).
- **Performance & Boundary Tests:** Dedicated tests for large datasets (streaming efficiency) and extreme numerical
  values (maximum precision and overflow checks).

### Type Safety

- **Decimal Arithmetic:** The project uses `rust_decimal` instead of floating-point types (`f32`/`f64`) to maintain
  absolute precision. While the specification expects up to 4 decimal places, the engine accurately preserves and
  processes higher precision inputs without rounding errors.
- **Domain Wrappers:** Types like `Balance` and `Amount` wrap `Decimal` to enforce domain rules (e.g., `Amount` must be
  positive) and prevent accidental misuse.

## Safety & Robustness

### Error Handling

- **Unified Error Strategy:** The project uses `thiserror` to define a single `PaymentError` enum that consolidates
  errors from all layers (Domain, Infrastructure, Application).
- **Rich Diagnostics:** `miette` is integrated at the CLI level to provide clear, actionable error reports and stack
  traces for developers and users.
- **Graceful Degradation:** Errors in individual transaction processing (e.g., malformed CSV rows) are logged to
  `stderr`, allowing the engine to continue processing subsequent valid transactions.

### Edge Case Management

- **Insufficient Funds for Dispute:** If a client attempts to dispute a transaction but lacks sufficient available funds
  to cover the hold (due to subsequent withdrawals), the dispute is rejected. This prevents available balances from
  becoming negative.
- **Duplicate Transactions:** The engine tracks transaction IDs and ignores duplicates to prevent double-spending or
  erroneous state updates.
- **Locked Accounts:** Once an account is locked (due to a chargeback), all subsequent transactions for that client (
  except further disputes/resolves) are automatically ignored.
- **Floating Point Safety:** By avoiding `f32`/`f64`, we eliminate the risk of precision-based financial discrepancies.

## Efficiency & Architecture

### Ordering Consistency

- **Guaranteed Sequential Order:** Transactions are processed in the exact order they are received from the CSV stream.
  This ensures strict chronological consistency for every client account, which is critical for correctly handling
  deposits, withdrawals, and the dispute lifecycle.
- **Lock-Free by Design:** Since transactions are processed sequentially by the engine, there is no need for complex
  global `Mutex` or `RwLock` structures for state management, eliminating lock contention.

### Scalability & High-Volume Processing

- **Supporting `u32::MAX` Transactions:**
    - **Constant Memory Footprint:** By utilizing streaming I/O, the engine processes transactions lazily without
      loading the entire dataset into RAM.
    - **Disk-Backed State:** The pluggable `RocksDB` backend allows the engine to manage transaction history and account
      states that exceed system memory, effectively scaling to the billions of records implied by `u32` transaction IDs.
- **Server & Network Readiness:**
    - **Async/Await Infrastructure:** The entire engine is built on the `tokio` async runtime. Every processing step and
      storage operation is non-blocking, making it highly suitable for integration into high-performance servers.
    - **Concurrent Stream Support:** The `TransactionReader` is generic over `std::io::Read`. In a networked context,
      thousands of concurrent `TcpStream` inputs could be handled simultaneously by spawning `tokio` tasks, with the
      async engine ensuring efficient resource utilization without thread-per-connection overhead.

### Design Decisions & Trade-offs

- **Streaming I/O:** The engine uses the standard `csv` crate to stream transactions from the input source. This keeps
  memory usage low regardless of the dataset size.
- **Why Direct Processing?** The architecture uses a direct async model where each transaction is processed and
  persisted immediately. Previous iterations used an Actor-based worker system, but this was removed to simplify the
  logic, as sharding/parallelism provided no benefit for the inherently sequential CSV input.
- **Why not `rayon`?** Parallelizing CSV reading with `rayon` was avoided because transactions for a specific client
  *must* be processed in the exact order they appear in the file.
- **Persistent Storage:** The engine includes a pluggable `AccountStore` and `TransactionStore` interface, with
  implementations for both `InMemory` (fast) and `RocksDB` (persistent), allowing it to handle datasets larger than
  available RAM.

## AI-Assisted Development Methodology

This project was developed with the assistance of an AI engineering partner. However, rather than using AI to "one-shot"
a solution, the methodology focused on iterative refinement and architectural integrity using the following principles:

- **Iterative TDD:** AI was used to help generate unit tests and property-based test cases based on the specs.
  Implementation was performed in small, verifiable steps.
- **Architectural Validation:** AI was used to brainstorm and validate architectural patterns (e.g., choosing the Actor
  model over global locks).
