use std::fs::File;

use crate::channel::Sender;
use crate::{aliases::*, channel::*};
use csv::ByteRecord;
use eyre::{eyre, Context, Result};
use rust_decimal::Decimal;
use std::ops::Deref;
use std::str::from_utf8;
use tracing::{debug, info};

enum RecordType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

pub struct CsvParser<T>(csv::Reader<T>);

impl<T: std::io::Read> CsvParser<T> {
    pub fn new(reader: T) -> CsvParser<T> {
        CsvParser(csv::Reader::from_reader(reader))
    }
}

impl CsvParser<File> {
    /// We will read file and parse each line. We assume spaces can be present in type and amount,
    /// other fields are assumed to be valid u16 and u32 for client and tx respectively
    /// Checking for whitespaces and their removal worsens the performance by roughly 1s per 10_000_000 records
    #[tracing::instrument(skip(self, transaction_sender, dispute_look_up_sender))]
    pub fn parse_journal(
        &mut self,
        transaction_sender: Sender<TransactionMessage>,
        dispute_look_up_sender: Sender<DisputeLookUpMessage>,
    ) -> Result<()> {
        info!("starting to parse transaction journal");
        let mut count = 0;

        let mut record_timer = std::time::Instant::now();
        for (index, record) in self.0.byte_records().enumerate() {
            if index % 10_000_000 == 0 {
                debug!(elapsed_seconds = record_timer.elapsed().as_secs(), %index, "processed 10_000_000 records");
                record_timer = std::time::Instant::now();
            }

            count = index;

            let record = record?;

            match parse_type(&record[0]) {
                // once we do not need to handle spaces, we can just match against bytes like record[0] == b"deposit"
                Ok(RecordType::Deposit) => {
                    let (client_id, _, amount) = parse_deposit_or_withdrawal(&record)?;
                    transaction_sender.send(TransactionMessage::deposit(client_id, amount));
                }
                Ok(RecordType::Withdrawal) => {
                    let (client_id, _, amount) = parse_deposit_or_withdrawal(&record)?;
                    transaction_sender.send(TransactionMessage::withdrawal(client_id, amount));
                }
                Ok(RecordType::Dispute) => {
                    let (client_id, transaction_id) = parse_dispute_data(&record)?;

                    debug!(%client_id, %transaction_id, %index, "found dispute transaction!");

                    dispute_look_up_sender
                        .send(DisputeLookUpMessage::Dispute(client_id, transaction_id));
                }
                Ok(RecordType::Resolve) => {
                    let (client_id, transaction_id) = parse_dispute_data(&record)?;
                    dispute_look_up_sender
                        .send(DisputeLookUpMessage::Resolve(client_id, transaction_id));
                }
                Ok(RecordType::Chargeback) => {
                    let (client_id, transaction_id) = parse_dispute_data(&record)?;
                    dispute_look_up_sender
                        .send(DisputeLookUpMessage::Chargeback(client_id, transaction_id));
                }
                _ => (),
            }
        }
        info!(%count, "finished parsing transaction journal");
        Ok(())
    }

    /// Goes through the file from the start and looks for requested transaction
    /// Stops when we reach transaction with ID higher than requested one or EOF or we find the requested transaction
    /// We check `client_id` and `transaction_id` to make sure we have correct transaction
    pub fn find_transaction(
        &mut self,
        client_id: ClientID,
        transaction_id: TransactionID,
    ) -> Result<(ClientID, TransactionID, Amount)> {
        // we should implement some logic to move to the closest position to the record we try to find
        // and not to start from the start everytime
        self.0.seek(csv::Position::new())?;
        for record in self.0.byte_records() {
            let record = record?;
            match &record[0] {
                b"withdrawal" | b"deposit" => {
                    let (found_client_id, found_transaction_id, amount) =
                        parse_deposit_or_withdrawal(&record)?;

                    if found_client_id == client_id && transaction_id == found_transaction_id {
                        return Ok((found_client_id, found_transaction_id, amount));
                    }

                    if found_transaction_id > transaction_id {
                        return Err(eyre!("Transaction for given dispute not found"));
                    }
                }
                _ => (),
            }
        }
        Err(eyre!(
            "transaction for requested client id and transaction id not found"
        ))
    }
}

fn parse_type(record: &[u8]) -> Result<RecordType> {
    if record.contains(&b' ') {
        let mut s = String::from(from_utf8(record).wrap_err("failed to read utf-8 from bytes")?);
        s.retain(|c| !c.is_ascii_whitespace());
        match s.as_str() {
            "deposit" => Ok(RecordType::Deposit),
            "withdrawal" => Ok(RecordType::Withdrawal),
            "dispute" => Ok(RecordType::Dispute),
            "resolve" => Ok(RecordType::Resolve),
            "chargeback" => Ok(RecordType::Chargeback),
            _ => Err(eyre!("invalid record type")),
        }
    } else {
        match record {
            b"deposit" => Ok(RecordType::Deposit),
            b"withdrawal" => Ok(RecordType::Withdrawal),
            b"dispute" => Ok(RecordType::Dispute),
            b"resolve" => Ok(RecordType::Resolve),
            b"chargeback" => Ok(RecordType::Chargeback),
            _ => Err(eyre!("invalid record type")),
        }
    }
}

fn parse_deposit_or_withdrawal(record: &ByteRecord) -> Result<(ClientID, TransactionID, Amount)> {
    let amount = match record[3].contains(&b' ') {
        true => Decimal::from_str_exact(
            from_utf8(
                &record[3]
                    .iter()
                    .filter_map(|b| {
                        if !b.is_ascii_whitespace() {
                            Some(b.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<u8>>()
                    .deref(),
            )
            .wrap_err("failed to parse amount to string")?,
        )
        .wrap_err("failed to convert str to decimal")?,
        false => Decimal::from_str_exact(
            from_utf8(&record[3]).wrap_err("failed to parse amount to string")?,
        )
        .wrap_err("failed to convert str to decimal")?,
    };

    Ok((
        from_utf8(&record[1])
            .expect("failed to parse client ID")
            .parse::<u16>()
            .wrap_err("failed to parse client id")?,
        from_utf8(&record[2])
            .expect("failed to parse transaction ID")
            .parse::<u32>()
            .wrap_err("failed to parse transaction id")?,
        amount,
    ))
}

fn parse_dispute_data(record: &ByteRecord) -> Result<(ClientID, TransactionID)> {
    Ok((
        from_utf8(&record[1])
            .wrap_err("failed to parse client ID")?
            .parse::<u16>()
            .wrap_err("failed to parse u16")?,
        from_utf8(&record[2])
            .wrap_err("failed to parse transaction ID")?
            .parse::<u32>()
            .wrap_err("failed to parse u32")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_deposit_or_withdrawal() {
        let tests: Vec<(&str, ByteRecord, (u16, u32, Decimal))> = vec![
            (
                "simple deposit",
                csv::ByteRecord::from(vec!["deposit", "1", "1", "1.0"]),
                (1, 1, dec!(1.0)),
            ),
            (
                "simple withdrawal",
                csv::ByteRecord::from(vec!["withdrawal", "1", "2", "1.0"]),
                (1, 2, dec!(1.0)),
            ),
            (
                "amount with space",
                csv::ByteRecord::from(vec!["deposit", "1", "3", "1. 0"]),
                (1, 3, dec!(1.0)),
            ),
            (
                "amount with multiple spaces",
                csv::ByteRecord::from(vec!["deposit", "1", "4", "10 . 0"]),
                (1, 4, dec!(10.0)),
            ),
        ];

        for (i, (name, test_data, want)) in tests.into_iter().enumerate() {
            let got = parse_deposit_or_withdrawal(&test_data).expect(&format!(
                "failed to parse data from ByteRecord for test {} - {name}",
                i + 1
            ));
            assert_eq!(got, want, "failed test {} - {name}", i + 1)
        }
    }
}
