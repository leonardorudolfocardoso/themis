use crate::amount::Amount;
use crate::command::Command;
use crate::event::Event;
use crate::projection::LedgerProjection;

pub(crate) enum Decision {
    Apply(Event),
    Ignore,
}

pub(crate) struct Decider;

impl Decider {
    pub(crate) fn decide(projection: &LedgerProjection, transaction: Command) -> Decision {
        match transaction {
            Command::Deposit { client, tx, amount } => {
                if projection.has_record(tx) {
                    Decision::Ignore
                } else {
                    Self::decide_deposit(projection, client, tx, amount)
                }
            }
            Command::Withdrawal { client, tx, amount } => {
                if projection.has_record(tx) {
                    Decision::Ignore
                } else {
                    Self::decide_withdrawal(projection, client, tx, amount)
                }
            }
            Command::Dispute { client, tx } => Self::decide_dispute(projection, client, tx),
            Command::Resolve { client, tx } => Self::decide_resolve(projection, client, tx),
            Command::Chargeback { client, tx } => Self::decide_chargeback(projection, client, tx),
        }
    }

    fn decide_deposit(
        projection: &LedgerProjection,
        client: u16,
        tx: u32,
        amount: Amount,
    ) -> Decision {
        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositAccepted { client, tx, amount })
    }

    fn decide_withdrawal(
        projection: &LedgerProjection,
        client: u16,
        tx: u32,
        amount: Amount,
    ) -> Decision {
        let Some(account) = projection.account(client) else {
            return Decision::Ignore;
        };

        if account.locked() || account.available() < amount {
            return Decision::Ignore;
        }

        Decision::Apply(Event::WithdrawalAccepted { client, tx, amount })
    }

    fn decide_dispute(projection: &LedgerProjection, client: u16, tx: u32) -> Decision {
        let amount = match projection.record(tx) {
            Some(record) if record.client == client && record.is_disputable() => record.amount,
            _ => return Decision::Ignore,
        };

        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositDisputed { client, tx, amount })
    }

    fn decide_resolve(projection: &LedgerProjection, client: u16, tx: u32) -> Decision {
        let amount = match projection.record(tx) {
            Some(record) if record.client == client && record.state.is_resolvable() => {
                record.amount
            }
            _ => return Decision::Ignore,
        };

        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DisputeResolved { client, tx, amount })
    }

    fn decide_chargeback(projection: &LedgerProjection, client: u16, tx: u32) -> Decision {
        let amount = match projection.record(tx) {
            Some(record) if record.client == client && record.state.is_chargebackable() => {
                record.amount
            }
            _ => return Decision::Ignore,
        };

        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositChargedBack { client, tx, amount })
    }
}
