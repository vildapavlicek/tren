use std::fs::OpenOptions;
use std::path::PathBuf;

use aliases::*;
use channel::{Dispute, DisputeLookUpMessage, TransactionMessage};
use tracing::{error, info, trace};

use crate::channel::Transaction;

mod accounts;
mod aliases;
mod channel;
mod dispute_look_up;
mod logger;
mod parser;
// mod transaction;

fn main() {
    let _guard = logger::init();

    info!(
        app_name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        "started journal parser"
    );

    let file_path: PathBuf = std::env::args()
        .nth(1)
        .expect("expected path to file to parse as and first argument, but got nothing")
        .into();

    let file_path_2 = file_path.clone();

    let start = std::time::Instant::now();

    let (transaction_sender, tx_receiver) =
        crossbeam_channel::bounded::<TransactionMessage>(10_000);

    let (dispute_look_up_sender, dispute_look_up_receiver) =
        crossbeam_channel::unbounded::<DisputeLookUpMessage>();

    let (transaction_sender, transaction_sender_2) = (
        channel::Sender::new(transaction_sender.clone()),
        channel::Sender::new(transaction_sender),
    );

    // parser thread
    std::thread::spawn(move || {
        parser::CsvParser::new(
            OpenOptions::new()
                .read(true)
                .open(file_path)
                .expect("failed to open file"),
        )
        .parse_journal(
            transaction_sender,
            channel::Sender::new(dispute_look_up_sender),
        )
    });

    // dispute look-up thread
    std::thread::spawn(move || {
        //  let mut dispute_cache: HashMap<TransactionID, Amount> = HashMap::new();
        dispute_look_up::DisputeFinder::new(
            OpenOptions::new()
                .read(true)
                .open(file_path_2)
                .expect("failed to open file"),
        )
        .run_dispute_look_up_loop(transaction_sender_2, dispute_look_up_receiver);
    });

    // transaction processing thread
    let handle = std::thread::spawn(move || {
        let mut accounts = accounts::Accounts::default();
        while let Ok(message) = tx_receiver.recv() {
            trace!(?message, "received ProcessTransactionMessage");
            match message {
                TransactionMessage::Deposit(Transaction { client_id, amount }) => {
                    accounts.deposit(client_id, amount)
                }
                TransactionMessage::Withdrawal(Transaction { client_id, amount }) => {
                    accounts.withdraw(client_id, amount)
                }
                TransactionMessage::Dispute(Dispute { client_id, amount }) => {
                    if let Err(err) = accounts.dispute(client_id, amount) {
                        error!(%err, "failed to do dispute");
                    }
                }
                TransactionMessage::Resolve(Dispute { client_id, amount }) => {
                    if let Err(err) = accounts.resolve(client_id, amount) {
                        error!(%err, "failed to do resolve");
                    }
                }
                TransactionMessage::Chargeback(Dispute { client_id, amount }) => {
                    if let Err(err) = accounts.chargeback(client_id, amount) {
                        error!(%err, "failed to do chargeback");
                    }
                }
            }
        }

        accounts
    });

    let result = handle.join();

    match result {
        Ok(accounts) => {
            info!(
                took_s = start.elapsed().as_secs(),
                "successfully finished processing journal"
            );
            accounts.print_report();
        }
        Err(err) => error!(?err, "failed to process transaction journal"),
    }
}
