use crate::aliases::*;
use eyre::{eyre, Result};
use rust_decimal::Decimal;
use serde::Deserializer;
use std::collections::HashMap;
use tracing::warn;

#[derive(Default)]
pub struct Accounts(HashMap<ClientID, AccountDetails>);

impl Accounts {
    /// Processes deposit done by the client, creates client's account if client doesn't have one yet
    /// # Arguments
    /// * client_id - used to look up client's [AccountDetails]
    /// * amount - value of how much client deposited
    pub fn deposit(&mut self, client_id: ClientID, amount: Decimal) {
        let acc_details = self
            .0
            .entry(client_id)
            .or_insert_with(AccountDetails::default);
        acc_details.deposit(amount)
    }

    /// Processes withdrawal done by the client, creates client's account if client doesn't have one yet
    /// # Arguments
    /// * client_id - used to look up client's [AccountDetails]
    /// * amount - value of how much client wants to withdraw
    pub fn withdraw(&mut self, client_id: ClientID, amount: Decimal) {
        let acc_details = self
            .0
            .entry(client_id)
            .or_insert_with(AccountDetails::default);

        if amount > acc_details.available {
            warn!(available = %acc_details.available, "client requested withdrawal with amount higher than available funds");
            return;
        }

        acc_details.withdraw(amount);
    }

    /// Handles dispute for given client and amount
    /// # Arguments
    /// * client_id - used to look up client's [AccountDetails]
    /// * amount - value of disputed transaction
    pub fn dispute(&mut self, client_id: ClientID, amount: Amount) -> Result<()> {
        match self.0.get_mut(&client_id) {
            Some(acc_details) => {
                acc_details.dispute(amount);
                Ok(())
            }
            None => Err(eyre!("cannot dispute transaction for non-existent account")),
        }
    }

    /// Resolves dispute for given client and amount
    /// # Arguments
    /// * client_id - used to look up client's [AccountDetails]
    /// * amount - value of disputed transaction
    pub fn resolve(&mut self, client_id: ClientID, amount: Amount) -> Result<()> {
        match self.0.get_mut(&client_id) {
            Some(acc_details) => {
                acc_details.resolve(amount);
                Ok(())
            }
            None => Err(eyre!(
                "cannot resolve disputed transaction for non-existent account"
            )),
        }
    }

    /// Does chargeback for provided client and amount
    /// # Arguments
    /// * client_id - used to look up client's [AccountDetails]
    /// * amount - value of disputed transaction
    pub fn chargeback(&mut self, client_id: ClientID, amount: Amount) -> Result<()> {
        match self.0.get_mut(&client_id) {
            Some(acc_details) => {
                acc_details.chargeback(amount);
                Ok(())
            }
            None => Err(eyre!(
                "cannot chargeback transaction for non-existent account"
            )),
        }
    }

    /// Prints out the report of all client's and their account state as described in requirements
    pub fn print_report(&self) {
        println!("client,available,held,total,locked");
        for (
            k,
            AccountDetails {
                account_status,
                total,
                available,
                held,
                ..
            },
        ) in self.0.iter()
        {
            println!(
                "{k},{available},{held},{total},{}",
                account_status.is_frozen()
            )
        }
    }
}

#[derive(PartialEq, Eq, Debug, serde::Deserialize)]
enum AccountStatus {
    Active,
    Frozen,
}

impl AccountStatus {
    pub fn is_frozen(&self) -> bool {
        matches!(self, AccountStatus::Frozen)
    }
}

#[derive(serde::Deserialize)]
/// AccountDetails encapsulates all relevant data for transaction processing. It tracks the values of `total`, `available` and `held` as well as account's status
struct AccountDetails {
    /// Status of the account, for example Active, Frozen etc.. See [AccountStatus] for possible values
    account_status: AccountStatus,
    #[serde(deserialize_with = "de_decimal")]
    total: Decimal,
    #[serde(deserialize_with = "de_decimal")]
    available: Decimal,
    #[serde(deserialize_with = "de_decimal")]
    held: Decimal,
}

fn de_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{Deserialize, Error, Unexpected};

    let s = String::deserialize(deserializer)?;
    Decimal::from_str_exact(&s).map_err(|err| {
        Error::invalid_value(
            Unexpected::Str(&s),
            &format!("valid Decimal, error parsing decimal '{err}'").as_str(),
        )
    })
}

impl AccountDetails {
    /// Increases `total` and `available` amounts
    /// # Arguments
    /// * amount - amount of the deposit which will be added to the total and available
    pub fn deposit(&mut self, amount: Decimal) {
        self.increase_balance(amount);
    }

    /// Decreases `total` and `available` amounts
    /// /// # Arguments
    /// * amount - amount of the withdrawal which will be subtracted from the total and available
    pub fn withdraw(&mut self, amount: Decimal) {
        // todo: we should probably deny withdrawal if the balance is not high enough
        self.decrease_balance(amount);
    }

    /// Does a dispute - increases `held` and decreases `availaible` by provided amount
    /// If found changes transactions state to [InDispute], moves it to in-dispute cache.
    /// # Arguments
    /// * amount - value of the disputed transaction
    pub fn dispute(&mut self, amount: Decimal) {
        self.held += amount;
        self.available -= amount;
    }

    /// Resolves dispute - reduces `held` and increaes `available` by amount provided
    /// If found changes transactions state to [Resolved], moves it to resolved cache.
    /// # Arguments
    /// * amount - value of the disputed transaction
    pub fn resolve(&mut self, amount: Decimal /* id: &TransactionID */) /* -> Result<()> */
    {
        self.held -= amount;
        self.available += amount;
    }

    /// Processes chargeback - decreases `held` and `total` and sets account's status to [AccountStatus::Frozen]
    /// If found changes transactions state to [Chargedback], moves it to chardeback cache.
    /// # Arguments
    /// * amount - value of the disputed transaction
    pub fn chargeback(&mut self, amount: Decimal /* id: &TransactionID */) /* -> Result<()> */
    {
        self.held -= amount;
        self.total -= amount;
        self.account_status = AccountStatus::Frozen;
    }

    #[inline(always)]
    fn increase_balance(&mut self, amount: Decimal) {
        self.total += amount;
        self.available += amount;
    }

    #[inline(always)]
    fn decrease_balance(&mut self, amount: Decimal) {
        self.total -= amount;
        self.available -= amount;
    }
}

impl Default for AccountDetails {
    fn default() -> Self {
        AccountDetails {
            account_status: AccountStatus::Active,
            total: Decimal::ZERO,
            available: Decimal::ZERO,
            held: Decimal::ZERO,
        }
    }
}
