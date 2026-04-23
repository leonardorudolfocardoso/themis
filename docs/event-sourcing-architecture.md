# Event Sourcing Architecture

This is the simplified target architecture for Themis.

The important distinction is:

- a `Command` is a request to do something;
- an `Event` is a fact Themis accepted;
- a `Projection` is state rebuilt from accepted events.

Themis should first implement this in memory. Durable streams, brokers, and
external databases can come later.

## Simple Flow

```mermaid
flowchart LR
    CSV[CSV / API / Broker] --> Command[Command]
    Command --> Ledger[Ledger / Decider]
    Projection[(Ledger Projection)] --> Ledger

    Ledger -->|accepted| EventLog[(Event Log)]
    Ledger -->|ignored| Ignored[Ignored Result]

    EventLog --> Projection
    Projection --> Output[CSV Output / Query API]
```

In words:

```text
Command -> decide -> Event -> project -> Account state
```

## Themis Parts

```mermaid
flowchart TD
    Command[Command<br/>Deposit, Withdrawal, Dispute, Resolve, Chargeback]

    Decider[Ledger / Decider<br/>checks rules against current projection]

    Event[Event<br/>Deposit, Withdrawal,<br/>DepositDisputed, DisputeResolved,<br/>DepositChargedBack]

    Projection[Ledger Projection<br/>accounts + transaction records]

    Command --> Decider
    Projection --> Decider
    Decider -->|accepted| Event
    Decider -->|ignored| Ignored[Ignored]
    Event --> Projection
```

The `Ledger` should answer one question:

```text
Given this command and the current ledger projection,
which accepted event should happen, if any?
```

The projection should answer a different question:

```text
Given this accepted event,
how does account and transaction state change?
```

## Example

```mermaid
sequenceDiagram
    participant Input as Input
    participant Ledger as Ledger / Decider
    participant Projection as Ledger Projection
    participant Log as Event Log

    Input->>Ledger: Command::Deposit(client=1, tx=10, amount=100)
    Ledger->>Projection: has tx=10? is account locked?
    Projection-->>Ledger: no duplicate, account open
    Ledger->>Log: append Event::Deposit
    Log->>Projection: apply Event::Deposit
    Projection-->>Projection: available += 100, record tx=10
```

## Current Implementation

The external input type is `Command`, and accepted ledger facts are `Event`s:

```rust
pub enum Event {
    Deposit { client, tx, amount },
    Withdrawal { client, tx, amount },
    DepositDisputed { client, tx, amount },
    DisputeResolved { client, tx, amount },
    DepositChargedBack { client, tx, amount },
}
```

`Ledger::apply(command)` now follows this shape:

```text
decide command
if accepted:
  append accepted event
  apply accepted event to projection
else:
  return Ignored
```

## What We Are Not Building Yet

Do not add Kafka, Redpanda, NATS, Postgres, or RocksDB yet.

Do not split account and transaction projections yet.

Do not optimize for multiple projection consumers yet.

The first goal is only this:

```text
Commands are requests.
Events are accepted facts.
Ledger state is a projection from accepted facts.
```
