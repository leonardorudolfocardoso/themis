# Architecture Rules

This project uses:
- CQRS
- Event Sourcing
- DDD aggregates

## Important constraints
- Events are the source of truth
- Commands must load aggregates from the event stream
- Read models are eventually consistent
- Avoid leaking read-model concerns into domain logic

## Workflow
Before implementing:
1. Explain aggregate boundaries
2. Validate consistency assumptions
3. Identify event versioning impacts
4. Propose migration strategy if schemas change

## Code preferences
- Favor explicit domain modeling
- Avoid framework magic
- Prefer immutable structures
