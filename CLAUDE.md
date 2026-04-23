# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`themis` is a financial transaction ledger in Rust (edition 2024) that reads CSV transaction commands from a file path and writes account state to stdout.

## Commands

```bash
cargo build          # compile
cargo run -- path/to/transactions.csv  # build and run
cargo test           # run all tests
cargo test test_deposit  # run a single test by name
cargo clippy         # lint
cargo fmt            # format
```

## Architecture

The ledger reads a CSV stream of transaction commands, decides which commands become accepted ledger events, projects accepted events into account state, and outputs account balances as CSV.

**Module structure:**
- `amount.rs` — `Amount(u64)` newtype for transaction values (always non-negative). Scale: 10,000 units per 1.0 (e.g. `1.2345` → `12345`). Constructed by parsing decimal strings from CSV; use `Amount::raw(u64)` in tests only.
- `funds.rs` — `Funds(i128)` newtype for account balances that can go negative (e.g. after chargeback following withdrawal). Supports `Add<Amount>`, `Sub<Amount>`, `AddAssign<Amount>`, `SubAssign<Amount>`, and `PartialOrd<Amount>`.
- `balance.rs` — `Balance` value object holding `available: Funds` and `held: Amount`. All monetary mutation logic lives here. `total()` is derived as `available + held`.
- `account.rs` — `Account` holds identity (`client: u16`), a `Balance`, and `locked: bool`. Locked check lives here; monetary operations delegate to `Balance`. Two distinct states: frozen funds (temporary, reversible) vs. locked account (permanent).
- `command.rs` — `Command` enum for external transaction requests (Deposit, Withdrawal, Dispute, Resolve, Chargeback).
- `event.rs` — `Event` enum for accepted ledger facts (Deposit, Withdrawal, DepositDisputed, DisputeResolved, DepositChargedBack).
- `decider.rs` — `Decider` checks each `Command` against the current `LedgerProjection` and returns either an accepted `Event` or `Ignore`.
- `event_log.rs` — In-memory append-only list of accepted ledger events.
- `projection.rs` — `LedgerProjection` applies accepted events to account balances and transaction records.
- `transaction.rs` — `Record`, `Kind`, and `State` track the dispute lifecycle of processed transactions. Only `Kind::Deposit` is disputable.
- `ledger.rs` — `Ledger` coordinates command handling: decide, append accepted events, project accepted events, and return final accounts.
- `csv/reader.rs` — Deserializes CSV rows into `RawEvent` via serde, converts to `Command` via `TryFrom`. Invalid rows are silently skipped.
- `csv/writer.rs` — Serializes `Account` iterator to CSV via serde using an `OutputRow` intermediate struct. Output is sorted by client ID.

**Key design decisions:**
- Amounts use integer arithmetic scaled by 10,000 — no floating point in parsing or domain logic.
- `Amount` (u64) vs `Funds` (i128): transaction values are always positive; account balances can go negative.
- `Command` and `Event` are deliberately separate: commands are external requests; events are accepted facts.
- `Decider` owns business rule decisions; `LedgerProjection` owns state changes from accepted events.
- `Balance` is a pure value object with no domain knowledge — `Account` owns the locked state and protects account invariants.
- `transaction::Record` tracks internal dispute state inside the projection.
- Disputes only apply to deposits, not withdrawals (`transaction::Kind` enforces this).
- Duplicate transaction IDs are silently ignored.
- Operations on locked accounts are silently ignored.
- All public items are documented; enforce with `RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps`.

## Commit style

Semantic prefixes: `feat:`, `fix:`, `refactor:`, `test:`, `chore:`. No co-author lines. Keep messages short.
