extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate csv;
extern crate chrono;
extern crate toml;

use std::io;
use chrono::naive::NaiveDate as DateTime;
use std::collections::HashMap;
use std::fs::File;

#[derive(Deserialize)]
struct Asset {
    filename: String,
    name: String,
    date_column: usize,
    date_format: String,
    amount_column: usize,
    #[serde(default)]
    separator: String,
}

impl Asset {
    fn deserialize_transaction(&self, record: &csv::StringRecord) -> io::Result<Transaction> {
        let date_str = record
            .get(self.date_column)
            .unwrap();

        // Returning error intentionally changed to unwrap,
        // so that finding the cause is easier
        let date = DateTime::parse_from_str(date_str
            /*.ok_or(io::ErrorKind::InvalidData)?*/, &self.date_format)
            .unwrap();
            //.map_err(|_| io::ErrorKind::InvalidData)?;
        let amount = record
            .get(self.amount_column)
            .unwrap()
            //.ok_or(io::ErrorKind::InvalidData)?
            .parse()
            .unwrap();
            //.map_err(|_| io::ErrorKind::InvalidData)?;

        Ok(Transaction {
            date,
            amount,
        })
    }
}

#[derive(Deserialize)]
struct Pair {
    filename: String,
    accounting_currency: String,
    price_of: String,
    price_column: usize,
    date_column: usize,
    date_format: String,
}

impl Pair {
    fn deserialize_price_change(&self, record: &csv::StringRecord) -> io::Result<PriceChange> {
        let date = DateTime::parse_from_str(record
            .get(self.date_column)
            .unwrap()
            /*.ok_or(io::ErrorKind::InvalidData)?*/, &self.date_format)
            .map_err(|_| io::ErrorKind::InvalidData)?;
        let new_price = record
            .get(self.price_column)
            .unwrap()
            //.ok_or(io::ErrorKind::InvalidData)?
            .parse()
            .unwrap();
            //.map_err(|_| io::ErrorKind::InvalidData)?;

        Ok(PriceChange {
            date,
            new_price,
        })
    }
}

#[derive(Deserialize)]
struct Configuration {
    asset: Vec<Asset>,
    pair: Vec<Pair>,
}

// Hopefully a better type one day
type Currency = f64;

struct Transaction {
    date: DateTime,
    amount: Currency,
}

struct PriceChange {
    date: DateTime,
    new_price: Currency,
}

struct PriceMap {
    map: HashMap<(String, String), Currency>,
}

impl PriceMap {
    fn update(&mut self, accounting: String, priced: String, price: Currency) {
        if accounting > priced {
            self.map.insert((priced, accounting), price);
        } else {
            self.map.insert((accounting, priced), 1.0 / price);
        }
    }

    fn convert(&self, currency: String, target: String, amount: Currency) -> Currency {
        if target > currency {
            amount * self.map.get(&(currency, target)).unwrap()
        } else {
            amount / self.map.get(&(target, currency)).unwrap()
        }
    }
}

enum Event {
    Transaction(usize),
    PriceChange(usize),
}

fn usage(program_name: &str) -> ! {
    println!("Usage: {} CONFIG_FILE UNIT_OF_ACCOUNT", program_name);
    std::process::exit(1)
}

fn main() -> std::io::Result<()> {
    use std::io::{Read, Write};

    let mut args = std::env::args();
    let program_name = args.next().expect("Not even zeroth argument given!");
    let config_file = args.next().unwrap_or_else(|| usage(&program_name));
    // Unit of account
    let uoa = args.next().unwrap_or_else(|| usage(&program_name));

    let mut config_data = Vec::new();
    std::fs::File::open(&config_file)?.read_to_end(&mut config_data)?;
    let config = toml::from_slice::<Configuration>(&config_data).unwrap();

    let mut tx_sources = config.asset
        .iter()
        .map(|asset| csv::ReaderBuilder::new().has_headers(false).delimiter(asset.separator.bytes().next().unwrap_or(b',')).from_path(&asset.filename))
        .map(|reader| reader.map(csv::Reader::into_records))
        .collect::<Result<Vec<_>, _>>()?;

    let mut price_sources = config.pair
        .iter()
        .map(|pair| csv::Reader::from_path(&pair.filename))
        .map(|reader| reader.map(csv::Reader::into_records))
        .collect::<Result<Vec<_>, _>>()?;

    let mut transactions = tx_sources
        .iter_mut()
        .map(Iterator::next)
        .map(|item| item.map(|item| item.map_err(io::Error::from)))
        .zip(&config.asset)
        .map(|(item, asset)| item.map(|item| item
             .and_then(|item| asset.deserialize_transaction(&item))))
        .map(|item| item.map_or(Ok(None), |v| v.map(Some)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut prices = price_sources
        .iter_mut()
        .map(Iterator::next)
        .map(|item| item.map(|item| item.map_err(io::Error::from)))
        .zip(&config.pair)
        .map(|(item, pair)| item.map(|item| item
             .and_then(|item| pair.deserialize_price_change(&item))))
        .map(|item| item.map_or(Ok(None), |v| v.map(Some)))
        .collect::<Result<Vec<_>, _>>()?;

    let mut price_map = PriceMap { map: HashMap::new() };

    for (price, pair) in prices.iter().zip(&config.pair) {
        price
            .as_ref()
            .map(|price| price_map.update(pair.accounting_currency.clone(), pair.price_of.clone(), price.new_price));
    }

    let mut balances = Vec::with_capacity(config.asset.len());
    balances.resize(config.asset.len(), 0.0);

    let mut maximalist_balances = Vec::with_capacity(config.asset.len());
    maximalist_balances.resize(config.asset.len(), 0.0);

    let mut transaction_log = File::create("transactions.txt")?;
    let mut balance_log = File::create("balances.txt")?;

    let mut balance_logs = config.asset
        .iter()
        .map(|asset| format!("balances-{}.txt", asset.name))
        .map(File::create)
        .collect::<Result<Vec<_>, _>>()?;

    let mut maximalist_balance_logs = config.asset
        .iter()
        .map(|asset| format!("maximalist-balances-{}.txt", asset.name))
        .map(File::create)
        .collect::<Result<Vec<_>, _>>()?;

    loop {
        let min_tx_date = transactions
            .iter()
            .map(|tx| tx.as_ref().map(|tx| tx.date))
            .enumerate()
            .fold(None, |min, (idx, item)| {
                match (min, item) {
                    (None, None) => None,
                    (Some(min), None) => Some(min),
                    (None, Some(item)) => Some((idx, item)),
                    (Some(min), Some(item)) => if min.1 < item {
                        Some(min)
                    } else {
                        Some((idx, item))
                    },
                }
            });

        let min_price_date = prices
            .iter()
            .map(|tx| tx.as_ref().map(|tx| tx.date))
            .enumerate()
            .fold(None, |min, (idx, item)| {
                match (min, item) {
                    (None, None) => None,
                    (Some(min), None) => Some(min),
                    (None, Some(item)) => Some((idx, item)),
                    (Some(min), Some(item)) => if min.1 < item {
                        Some(min)
                    } else {
                        Some((idx, item))
                    },
                }
            });

        let event = match (min_tx_date, min_price_date) {
            (None, None) => break,
            (Some((idx, _)), None) => Event::Transaction(idx),
            (None, Some((idx, _))) => Event::PriceChange(idx),
            (Some((idx0, date0)), Some((idx1, date1))) => if date0 < date1 {
                Event::Transaction(idx0)
            } else {
                Event::PriceChange(idx1)
            },
        };

        let evt_date = match event {
            Event::Transaction(idx) => {
                let this_asset = &config.asset[idx];

                let new = tx_sources[idx]
                    .next()
                    .map(|item| item.map_err(io::Error::from))
                    .map(|item| item.and_then(|item| this_asset.deserialize_transaction(&item)))
                    .map_or(Ok(None), |v| v.map(Some))?;
                let transaction = std::mem::replace(&mut transactions[idx], new).unwrap();
                balances[idx] += transaction.amount;
                let amount = if this_asset.name == uoa {
                    transaction.amount
                } else {
                    price_map.convert(this_asset.name.clone(), uoa.clone(), transaction.amount)
                };
                writeln!(transaction_log, "{} {}", transaction.date, amount)?;

                for (balance, asset) in maximalist_balances.iter_mut().zip(&config.asset) {
                    if asset.name == this_asset.name {
                        *balance += transaction.amount;
                    } else {
                        *balance += price_map.convert(this_asset.name.clone(), asset.name.clone(), transaction.amount);
                    }
                }

                transaction.date
            },
            Event::PriceChange(idx) => {
                let this_pair = &config.pair[idx];

                let new = price_sources[idx]
                    .next()
                    .map(|item| item.map_err(io::Error::from))
                    .map(|item| item.and_then(|item| this_pair.deserialize_price_change(&item)))
                    .map_or(Ok(None), |v| v.map(Some))?;
                let price_change = std::mem::replace(&mut prices[idx], new).unwrap();
                let pair = &this_pair;
                price_map.update(pair.accounting_currency.clone(), pair.price_of.clone(), price_change.new_price);
                price_change.date
            }
        };

        let total: Currency = balances
            .iter()
            .zip(&config.asset)
            .zip(&mut balance_logs)
            .zip(&maximalist_balances)
            .zip(&mut maximalist_balance_logs)
            .map(|((((balance, asset), asset_log), maximalist_balance), maximalist_balance_log)| {
                let balance = if asset.name == uoa {
                    *balance
                } else {
                    price_map.convert(asset.name.clone(), uoa.clone(), *balance)
                };

                let maximalist_balance = if asset.name == uoa {
                    *maximalist_balance
                } else {
                    price_map.convert(asset.name.clone(), uoa.clone(), *maximalist_balance)
                };

                writeln!(asset_log, "{} {}", evt_date, balance).unwrap();
                writeln!(maximalist_balance_log, "{} {}", evt_date, maximalist_balance).unwrap();

                balance
            })
            .sum();

        writeln!(balance_log, "{} {}", evt_date, total)?;
    }

    Ok(())
}
