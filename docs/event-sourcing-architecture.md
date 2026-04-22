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
    Command --> Processor[Processor / Decider]
    Projection[(Ledger Projection)] --> Processor

    Processor -->|accepted| EventLog[(Event Log)]
    Processor -->|ignored| Ignored[Ignored Result]

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

    Decider[Processor / Decider<br/>checks rules against current projection]

    Event[Event<br/>DepositAccepted, WithdrawalAccepted,<br/>DepositDisputed, DisputeResolved,<br/>DepositChargedBack]

    Projection[Ledger Projection<br/>accounts + transaction records]

    Command --> Decider
    Projection --> Decider
    Decider -->|accepted| Event
    Decider -->|ignored| Ignored[Ignored]
    Event --> Projection
```

The `Processor` should answer one question:

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
    participant Processor as Processor / Decider
    participant Projection as Ledger Projection
    participant Log as Event Log

    Input->>Processor: Command::Deposit(client=1, tx=10, amount=100)
    Processor->>Projection: has tx=10? is account locked?
    Projection-->>Processor: no duplicate, account open
    Processor->>Log: append Event::DepositAccepted
    Log->>Projection: apply Event::DepositAccepted
    Projection-->>Projection: available += 100, record tx=10
```

## Current Code Direction

The current code already moved the external input type to `Command`.

The next useful code step is to introduce accepted domain events:

```rust
pub enum Event {
    DepositAccepted { client, tx, amount },
    WithdrawalAccepted { client, tx, amount },
    DepositDisputed { client, tx, amount },
    DisputeResolved { client, tx, amount },
    DepositChargedBack { client, tx, amount },
}
```

After that, `Processor::apply(command)` can become:

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
