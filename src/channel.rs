use crate::{Amount, ClientID, TransactionID};
use rust_decimal::Decimal;
use std::fmt::Debug;
use tracing::{error, trace};

/// Helper wrapper around channel with only `send`  method.
pub struct Sender<T>(crossbeam_channel::Sender<T>);

impl<T: Debug> Sender<T> {
    pub fn new(sender: crossbeam_channel::Sender<T>) -> Self {
        Sender(sender)
    }

    /// Handles internally errors so we don't get warning all over the code base about not used errors
    pub fn send(&self, message: T) {
        match self.0.send(message) {
            Ok(_) => trace!("successfully send message over channel"),
            Err(err) => error!(%err, "failed to send message over channel"),
        }
    }
}

#[derive(Debug)]
pub struct Transaction {
    pub client_id: ClientID,
    pub amount: Amount,
}

impl Transaction {
    pub fn new(client_id: u16, amount: Decimal) -> Self {
        Transaction { amount, client_id }
    }
}

#[derive(Debug)]
pub struct Dispute {
    pub client_id: ClientID,
    pub amount: Amount,
}

impl Dispute {
    #[inline(always)]
    pub fn new(client_id: u16, amount: Decimal) -> Self {
        Dispute { client_id, amount }
    }
}

#[derive(Debug)]
pub enum TransactionMessage {
    Deposit(Transaction),
    Withdrawal(Transaction),
    Dispute(Dispute),
    Resolve(Dispute),
    Chargeback(Dispute),
}

impl TransactionMessage {
    pub fn deposit(client_id: ClientID, amount: Amount) -> Self {
        Self::Deposit(Transaction::new(client_id, amount))
    }
    pub fn withdrawal(client_id: ClientID, amount: Amount) -> Self {
        Self::Withdrawal(Transaction::new(client_id, amount))
    }
    pub fn dispute(client_id: ClientID, amount: Amount) -> Self {
        Self::Dispute(Dispute::new(client_id, amount))
    }
    pub fn resolve(client_id: ClientID, amount: Amount) -> Self {
        Self::Resolve(Dispute::new(client_id, amount))
    }
    pub fn chargeback(client_id: ClientID, amount: Amount) -> Self {
        Self::Chargeback(Dispute::new(client_id, amount))
    }
}

#[derive(Debug)]
pub enum DisputeLookUpMessage {
    Dispute(ClientID, TransactionID),
    Resolve(ClientID, TransactionID),
    Chargeback(ClientID, TransactionID),
}

impl DisputeLookUpMessage {
    pub fn client_id(&self) -> u16 {
        match self {
            Self::Dispute(client_id, _)
            | Self::Resolve(client_id, _)
            | Self::Chargeback(client_id, _) => *client_id,
        }
    }

    pub fn transaction_id(&self) -> u32 {
        match self {
            Self::Dispute(_, transaction_id)
            | Self::Resolve(_, transaction_id)
            | Self::Chargeback(_, transaction_id) => *transaction_id,
        }
    }
}
