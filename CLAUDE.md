# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`themis` is a financial transaction processor in Rust (edition 2024) that reads CSV transaction events from a file path and writes account state to stdout.

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

The processor reads a CSV stream of transaction events, applies them to accounts, and outputs account balances as CSV.

**Module structure:**
- `amount.rs` — `Amount(u64)` newtype for transaction values (always non-negative). Scale: 10,000 units per 1.0 (e.g. `1.2345` → `12345`). Constructed by parsing decimal strings from CSV; use `Amount::raw(u64)` in tests only.
- `funds.rs` — `Funds(i128)` newtype for account balances that can go negative (e.g. after chargeback following withdrawal). Supports `Add<Amount>`, `Sub<Amount>`, `AddAssign<Amount>`, `SubAssign<Amount>`, and `PartialOrd<Amount>`.
- `balance.rs` — `Balance` value object holding `available: Funds` and `held: Amount`. All monetary mutation logic lives here. `total()` is derived as `available + held`.
- `account.rs` — `Account` holds identity (`client: u16`), a `Balance`, and `locked: bool`. Locked check lives here; monetary operations delegate to `Balance`. Two distinct states: frozen funds (temporary, reversible) vs. locked account (permanent).
- `event.rs` — `Event` enum for all transaction input operations (Deposit, Withdrawal, Dispute, Resolve, Chargeback).
- `transaction.rs` — `Record`, `Kind`, and `State` track the dispute lifecycle of processed transactions. Only `Kind::Deposit` is disputable.
- `processor.rs` — `Processor` consumes an `Iterator<Item = Event>` and returns `HashMap<u16, Account>`.
- `csv/reader.rs` — Deserializes CSV rows into `RawEvent` via serde, converts to `Event` via `TryFrom`. Invalid rows are silently skipped.
- `csv/writer.rs` — Serializes `Account` iterator to CSV via serde using an `OutputRow` intermediate struct. Output is sorted by client ID.

**Key design decisions:**
- Amounts use integer arithmetic scaled by 10,000 — no floating point in parsing or domain logic.
- `Amount` (u64) vs `Funds` (i128): transaction values are always positive; account balances can go negative.
- `Balance` is a pure value object with no domain knowledge — `Account` owns the locked state and guards deposit/withdraw.
- `transaction::Record` is separate from `Event`: events are input, records track internal dispute state.
- Disputes only apply to deposits, not withdrawals (`transaction::Kind` enforces this).
- Duplicate transaction IDs are silently ignored.
- Operations on locked accounts are silently ignored.
- All public items are documented; enforce with `RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps`.

## Commit style

Semantic prefixes: `feat:`, `fix:`, `refactor:`, `test:`, `chore:`. No co-author lines. Keep messages short.
