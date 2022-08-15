/* use crate::aliases::*;
use rust_decimal::Decimal;

/// This struct represents single entry (row) in csv file
#[derive(serde::Deserialize, Debug, PartialEq, Eq)]
pub struct JournalEntry {
    #[serde(deserialize_with = "de_tx_type")]
    pub r#type: EntryType,
    #[serde(deserialize_with = "de_u16")]
    pub client: ClientID,
    #[serde(deserialize_with = "de_u32")]
    pub tx: TransactionID,
    #[serde(deserialize_with = "de_decimal")]
    pub amount: Option<Decimal>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EntryType {
    Deposit,
    Withdrawal,
    Resolve,
    ChargeBack,
    Dispute,
}

/**
 * Because it was said to accept whitespaces, we have to make our own deserialization function where we remove them
 * CSV with whitespaces (except in strings) is deemed invalid and parsing fails / returns error
 **/
use serde::{
    de::{Deserialize, Error, Unexpected},
    Deserializer,
};

fn de_tx_type<'de, D>(deserializer: D) -> Result<EntryType, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s = String::deserialize(deserializer)?;
    remove_whitespaces(&mut s);
    match s.as_str() {
        "deposit" => Ok(EntryType::Deposit),
        "withdrawal" => Ok(EntryType::Withdrawal),
        "resolve" => Ok(EntryType::Resolve),
        "chargeback" => Ok(EntryType::ChargeBack),
        "dispute" => Ok(EntryType::Dispute),
        _ => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(&s),
            &"expect one of the: deposit | withdrawal | resolve | chargeback | dispute",
        )),
    }
}

fn de_u16<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s = String::deserialize(deserializer)?;
    remove_whitespaces(&mut s);
    s.parse::<u16>().map_err(|err| {
        Error::invalid_value(
            Unexpected::Str(&s),
            &format!("valid u16, error parsing u16 '{err}'").as_str(),
        )
    })
}

fn de_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let mut s = String::deserialize(deserializer)?;
    remove_whitespaces(&mut s);
    s.parse::<u32>().map_err(|err| {
        Error::invalid_value(
            Unexpected::Str(&s),
            &format!("valid u32, error parsing u32: '{err}'").as_str(),
        )
    })
}

fn de_decimal<'de, D>(deserializer: D) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(mut s) => {
            s.retain(|c| !c.is_whitespace());
            Ok(Some(Decimal::from_str_exact(&s).map_err(|err| {
                Error::invalid_value(
                    Unexpected::Str(&s),
                    &format!("valid Decimal, error parsing decimal '{err}'").as_str(),
                )
            })?))
        }
        None => Ok(None),
    }
}

fn remove_whitespaces(s: &mut String) {
    s.retain(|c| !c.is_ascii_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn deserialize() {
        let data = r#"type,client,tx,amount
        deposit,1,1,10000
        deposit,1,2,10000.0
        deposit, 1, 3, 10000.0
        deposit, 1, 4, 10 000.0
        deposit, 1, 5 0, 10 000.0
        deposit, 1, 6 0, 10 000.0000
        deposit, 0 1, 7 0, 10 000.0000
        withdrawal, 1, 8,10000.0
        chargeback, 1, 9,
        resolve, 1, 10,
        dispute, 1, 11,"#;

        let want = vec![
            JournalEntry {
                r#type: EntryType::Deposit,
                client: 1,
                tx: 1,
                amount: Some(Decimal::new(10_000, 0)),
            },
            JournalEntry {
                r#type: EntryType::Deposit,
                client: 1,
                tx: 2,
                amount: Some(Decimal::new(100_000, 1)),
            },
            JournalEntry {
                r#type: EntryType::Deposit,
                client: 1,
                tx: 3,
                amount: Some(Decimal::new(100_000, 1)),
            },
            JournalEntry {
                r#type: EntryType::Deposit,
                client: 1,
                tx: 4,
                amount: Some(Decimal::new(100_000, 1)),
            },
            JournalEntry {
                r#type: EntryType::Deposit,
                client: 1,
                tx: 50,
                amount: Some(Decimal::new(100_000, 1)),
            },
            JournalEntry {
                r#type: EntryType::Deposit,
                client: 1,
                tx: 60,
                amount: Some(Decimal::new(10_000_0000, 4)),
            },
            JournalEntry {
                r#type: EntryType::Deposit,
                client: 1,
                tx: 70,
                amount: Some(Decimal::new(10_000_0000, 4)),
            },
            JournalEntry {
                r#type: EntryType::Withdrawal,
                client: 1,
                tx: 8,
                amount: Some(Decimal::new(10_000_0, 1)),
            },
            JournalEntry {
                r#type: EntryType::ChargeBack,
                client: 1,
                tx: 9,
                amount: None,
            },
            JournalEntry {
                r#type: EntryType::Resolve,
                client: 1,
                tx: 10,
                amount: None,
            },
            JournalEntry {
                r#type: EntryType::Dispute,
                client: 1,
                tx: 11,
                amount: None,
            },
        ];

        let mut reader = csv::Reader::from_reader(data.as_bytes());

        for (got, want) in reader.deserialize::<JournalEntry>().zip(want) {
            assert_eq!(
                got.expect("failed to deserialize test data"),
                want,
                "failed test case for transaction {}",
                want.tx
            )
        }
    }
}
 */
