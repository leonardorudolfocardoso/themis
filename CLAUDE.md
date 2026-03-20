# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`themis` is a financial transaction processor in Rust (edition 2024) that reads CSV transaction events from stdin and writes account state to stdout.

## Commands

```bash
cargo build          # compile
cargo run            # build and run (reads CSV from stdin)
cargo test           # run all tests
cargo test test_deposit  # run a single test by name
cargo clippy         # lint
cargo fmt            # format
```

## Architecture

The processor reads a CSV stream of transaction events, applies them to accounts, and outputs account balances as CSV.

**Module structure:**
- `amount.rs` — `Amount(u64)` newtype for transaction values (always non-negative). Scale: 10,000 units per 1.0 (e.g. `1.2345` → `12345`). Constructed via `From<u64>` internally or `TryFrom<f64>` from CSV.
- `funds.rs` — `Funds(i64)` newtype for account balances that can go negative (e.g. after chargeback following withdrawal). Supports `Add<Amount>`, `Sub<Amount>`, `AddAssign<Amount>`, `SubAssign<Amount>`, and `PartialOrd<Amount>`.
- `balance.rs` — `Balance` value object holding `available: Funds` and `held: Amount`. All monetary mutation logic lives here. `total()` is derived as `available + held`.
- `account.rs` — `Account` holds identity (`client: u16`), a `Balance`, and `locked: bool`. Locked check lives here; monetary operations delegate to `Balance`.
- `event.rs` — `Event` enum for all transaction operations. `TransactionRecord` tracks dispute lifecycle with `TransactionKind` (only `Deposit` is disputable) and `TransactionState`.
- `processor.rs` — `Processor` consumes an `Iterator<Item = Event>` and returns `HashMap<u16, Account>`.
- `csv/reader.rs` — Deserializes CSV rows into `RawEvent` via serde, converts to `Event` via `TryFrom`.
- `csv/writer.rs` — Serializes `Account` iterator to CSV via serde using an `OutputRow` intermediate struct.

**Key design decisions:**
- Amounts use integer arithmetic scaled by 10,000 — no floating point in domain logic.
- `Amount` (u64) vs `Funds` (i64): transaction values are always positive; account balances can go negative.
- `Balance` is a pure value object — `Account` owns the locked state and guards deposit/withdraw.
- Disputes only apply to deposits, not withdrawals (`TransactionKind` enforces this).
- Duplicate transaction IDs are silently ignored.
- Operations on locked accounts are silently ignored.

## Commit style

Semantic prefixes: `feat:`, `fix:`, `refactor:`, `test:`, `chore:`. No co-author lines. Keep messages short.
