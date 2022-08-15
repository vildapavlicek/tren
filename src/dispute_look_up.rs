use crate::channel::Sender;
use crate::{aliases::*, parser, DisputeLookUpMessage, TransactionMessage};
use crossbeam_channel::Receiver;
use eyre::{eyre, Result};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fs::File;
use tracing::{debug, error, trace};

// dispute finder should have some kind of caching mechanism to speed up search times for big files
// we could for example cache position for every 10_000th transaction so then we could quicly move to closest postion
// instead of starting from beginning of the file
pub struct DisputeFinder<T> {
    parser: parser::CsvParser<T>,
    cache: HashMap<TransactionID, Amount>,
}

impl<T: std::io::Read> DisputeFinder<T> {
    pub fn new(reader: T) -> DisputeFinder<T> {
        DisputeFinder {
            parser: parser::CsvParser::new(reader),
            cache: HashMap::new(),
        }
    }
}

impl DisputeFinder<File> {
    #[tracing::instrument(skip(self))]
    pub fn find_dispute_amount(
        &mut self,
        client_id: ClientID,
        transaction_id: TransactionID,
    ) -> Result<Decimal> {
        if let Some(amount) = self.cache.get(&transaction_id) {
            debug!(%amount, "found disputed transaction in cache");
            return Ok(amount.clone());
        }

        debug!("dispute transaction not found in cache, will search in file");
        let amount = self
            .parser
            .find_transaction(client_id, transaction_id)
            .map(|r| r.2)?;

        trace!("disputed transaction found");
        self.cache.insert(transaction_id, amount.clone());
        Ok(amount)
    }

    pub fn remove_from_cache(&mut self, transaction_id: TransactionID) -> Result<Amount> {
        self.cache
            .remove(&transaction_id)
            .ok_or(eyre!("value not found in cache, failed to remove"))
    }

    #[tracing::instrument(skip(self, sender, receiver))]
    pub fn run_dispute_look_up_loop(
        mut self,
        sender: Sender<TransactionMessage>,
        receiver: Receiver<DisputeLookUpMessage>,
    ) {
        while let Ok(look_up_request) = receiver.recv() {
            let span = tracing::trace_span!(
                "look_up_request",
                client_id = look_up_request.client_id(),
                transaction_id = look_up_request.transaction_id()
            );

            let _enter = span.enter();
            debug!(?look_up_request, "received dispute look-up request");

            match look_up_request {
                DisputeLookUpMessage::Dispute(client_id, transaction_id) => {
                    match self.find_dispute_amount(client_id, transaction_id) {
                        Ok(amount) => {
                            sender.send(TransactionMessage::dispute(client_id, amount));
                        }
                        Err(err) => error!(%err, "failed to find disputed transaction"),
                    };
                }
                DisputeLookUpMessage::Resolve(client_id, transaction_id) => {
                    match self.find_dispute_amount(client_id, transaction_id) {
                        Ok(amount) => {
                            sender.send(TransactionMessage::resolve(client_id, amount));
                            self.remove_from_cache(transaction_id);
                        }
                        Err(err) => error!(%err, "failed to find disputed transaction"),
                    }
                }
                DisputeLookUpMessage::Chargeback(client_id, transaction_id) => {
                    match self.find_dispute_amount(client_id, transaction_id) {
                        Ok(amount) => {
                            sender.send(TransactionMessage::chargeback(client_id, amount));
                            self.remove_from_cache(transaction_id);
                        }
                        Err(err) => error!(%err, "failed to find disputed transaction"),
                    }
                }
            };
        }
    }
}
