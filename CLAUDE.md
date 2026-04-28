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

The ledger reads a CSV stream of transaction commands, validates them, emits events, and outputs account balances as CSV. The design follows an event-sourcing pattern: events are the source of truth, and state is derived by replaying them.

**Command → Event flow:**
1. `Ledger::ingest` receives commands from any source (CSV, future server, etc.)
2. `Ledger::decide` validates each command against both aggregates (immutable borrow)
3. `Ledger::record` appends the approved event to the `Log` and applies it to both aggregates
4. `Ledger::replay` rebuilds state from a `Log` without validation

**Module structure:**
- `amount.rs` — `Amount(u64)` newtype for transaction values (always non-negative). Scale: 10,000 units per 1.0 (e.g. `1.2345` → `12345`). Constructed by parsing decimal strings from CSV; use `Amount::raw(u64)` in tests only.
- `funds.rs` — `Funds(i128)` newtype for account balances that can go negative (e.g. after chargeback following withdrawal). Supports `Add<Amount>`, `Sub<Amount>`, `AddAssign<Amount>`, `SubAssign<Amount>`, and `PartialOrd<Amount>`.
- `balance.rs` — `Balance` value object holding `available: Funds` and `held: Amount`. All monetary mutation logic lives here. `total()` is derived as `available + held`.
- `account.rs` — `Account` holds identity (`client: u16`), a `Balance`, and `locked: bool`. Locked check lives here; monetary operations delegate to `Balance`. `Accounts` is a newtype over `HashMap<ClientId, Account>` — the account aggregate. Validates locked state and withdrawal eligibility; applies account-related events.
- `command.rs` — `Command` enum for all transaction input operations (Deposit, Withdrawal, Dispute, Resolve, Chargeback).
- `event.rs` — `Event` enum (Deposited, Withdrawn, DisputeOpened, DisputeResolved, ChargedBack), `Decision` enum (Approved/Denied), and `Log` newtype over `Vec<Event>`.
- `transaction.rs` — `Transaction`, `Kind`, and `State` track the dispute lifecycle. `Transactions` is a newtype over `HashMap<TransactionId, Transaction>` — the transaction aggregate. Validates tx identity, client ownership, and dispute lifecycle; applies transaction-related events.
- `ledger.rs` — `Ledger` orchestrates: owns `Log`, `Transactions`, and `Accounts`. `decide` queries both aggregates, `record` appends to log and fans out to both.
- `csv/reader.rs` — Deserializes CSV rows into `RawCommand` via serde, converts to `Command` via `TryFrom`. Invalid rows are silently skipped.
- `csv/writer.rs` — Serializes `Accounts` to CSV via serde using an `OutputRow` intermediate struct. Output is sorted by client ID.

**Key design decisions:**
- Amounts use integer arithmetic scaled by 10,000 — no floating point in parsing or domain logic.
- `Amount` (u64) vs `Funds` (i128): transaction values are always positive; account balances can go negative.
- `Balance` is a pure value object with no domain knowledge — `Account` owns the locked state and guards deposit/withdraw.
- Event sourcing: `Log` is the source of truth. `Transactions` and `Accounts` are projections rebuilt by applying events. `replay` proves the log is sufficient to reconstruct all state.
- Two aggregates with encapsulated validation: `Transactions` owns dispute lifecycle rules, `Accounts` owns balance/locked rules. `Ledger.decide` composes both — no direct field access into aggregate internals.
- Commands are input, events are output of validation. `Transaction` (internal to aggregate) tracks dispute state and is separate from both.
- Disputes only apply to deposits, not withdrawals (`transaction::Kind` enforces this).
- Duplicate transaction IDs are silently ignored.
- Operations on locked accounts are silently ignored.
- All public items are documented; enforce with `RUSTDOCFLAGS="-D missing_docs" cargo doc --no-deps`.

## Commit style

Semantic prefixes: `feat:`, `fix:`, `refactor:`, `test:`, `chore:`. No co-author lines. Keep messages short.
