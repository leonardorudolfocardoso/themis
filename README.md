# Themis

> In Greek mythology, Themis is the goddess of justice, law, and divine order — the keeper of rules that cannot be broken.

Themis is a financial transaction ledger. It reads a stream of transaction commands from a CSV file, applies them to client accounts, and outputs the resulting account state. Like its namesake, it enforces rules that cannot be bent: a locked account stays locked, a withdrawal dispute is ignored, a chargeback that leaves a balance negative is still recorded faithfully.

## Usage

Output is written to stdout as CSV.

Example:

```bash
cargo run -- tests/fixtures/transactions.csv
```

## Input format

A CSV file with the following columns:

| Column   | Type    | Description                        |
|----------|---------|------------------------------------|
| `type`   | string  | `deposit`, `withdrawal`, `dispute`, `resolve`, `chargeback` |
| `client` | u16     | Client ID                          |
| `tx`     | u32     | Transaction ID (globally unique)   |
| `amount` | decimal string | Non-negative amount with up to 4 decimal places; omitted for dispute/resolve/chargeback |

Example:

```csv
type,client,tx,amount
deposit,1,1,100.0
withdrawal,1,2,30.0
dispute,1,1,
resolve,1,1,
```

## Output format

A CSV with one row per client:

| Column      | Description                              |
|-------------|------------------------------------------|
| `client`    | Client ID                                |
| `available` | Funds available for withdrawal           |
| `held`      | Funds held under dispute                 |
| `total`     | `available + held`                       |
| `locked`    | `true` if account was charged back       |

## Rules

- Duplicate transaction IDs are silently ignored.
- Only deposits can be disputed — disputes on withdrawals are ignored.
- Disputes, resolves, and chargebacks must reference a transaction belonging to the same client.
- Operations on locked accounts are silently ignored.
- A chargeback after a withdrawal can result in a negative balance — the account owes the bank.

## Implementation Notes

- Monetary values are parsed directly from decimal strings and stored as scaled integers with 4 decimal places. Account logic uses integer arithmetic only.
- `Amount` represents non-negative transaction values, while account balances use a signed `Funds` type so chargebacks can drive totals below zero.
- Account locking is enforced separately from balance math. This keeps the balance model simple and makes the "locked accounts ignore future operations" rule easy to reason about.
- Event sourcing: commands are validated (`decide`) and produce events that are appended to a `Log`. The `Transactions` and `Accounts` aggregates are projections rebuilt by applying events. `Ledger::replay` proves the log is the sole source of truth.
- Each aggregate encapsulates its own validation rules. The ledger composes their answers without reaching into their internals.

## Assumptions

- Invalid CSV rows are skipped rather than terminating the program.
- Malformed, negative, over-precise, or overflowing amounts are treated as invalid rows.
- Amounts are expected to fit within the numeric range supported by the implementation.
- Inputs with more than 4 decimal places are rejected rather than rounded.
- The output is sorted by client ID to keep runs deterministic and reviewer-friendly.

## Architecture

```mermaid
sequenceDiagram
    participant CSV as CSV Input
    participant Reader as csv::Reader
    participant Ledger
    participant Txns as Transactions
    participant Accts as Accounts
    participant Log
    participant Writer as csv::Writer
    participant Out as CSV Output

    CSV->>Reader: raw row
    Reader->>Ledger: Command (or skip invalid row)

    Note over Ledger: decide (immutable)
    Ledger->>Txns: validate (duplicate? disputable? client match?)
    Ledger->>Accts: validate (locked? sufficient funds?)
    Note over Ledger: Decision::Approved(Event)

    Note over Ledger: record
    Ledger->>Log: push(event)
    Ledger->>Txns: apply(event)
    Ledger->>Accts: apply(event)

    Note over Ledger: output
    Ledger->>Writer: Accounts
    Writer->>Writer: sort by client ID
    Writer->>Out: client, available, held, total, locked

    Note over Log: replay rebuilds state
    Log->>Ledger: Ledger::replay(log)
```

## Development

```bash
cargo test       # run all tests
cargo clippy     # lint
cargo fmt        # format
```

## AI Disclosure

AI tools used: OpenAI Codex and Anthropic Claude.

They were used as pair-programming and review aids to sanity-check
implementation details, review documentation, and help surface edge cases and
wording issues. Final technical decisions, validation, and submitted changes
were my own.
