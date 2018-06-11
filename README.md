Plot your savings in multiple currencies
========================================

This is a tool for plotting of savings when one saves in multiple currencies
or assets.  A popular example is using Bitcoin and some fiat currency (USD,
EUR...)

Warning/disclaimer
------------------

This software is provided "as is", with no guarantees nor warranty! It was
created in good faith to help people but the author takes no responsibility
for its behavior, especially when it comes to making financial decisions.

Use at your own risk!

Further, any examples here aren't financial advice. While the author likes
Bitcoin, he doesn't offer any guarantees when it comes to trading or financial
management!

Use your due dilligence!

How to use
----------

The tool is written mostly in Rust language but needs gnuplot to do the plotting.
So you need to install Rust compiler (with its build system `cargo`) and gnuplot.

First, you need the data about your accounts (wallets) and data about prices of
the assets you own.

Then you have to write a configuration file, which is a toml. Example:

```toml
# Specifies an asset you use.
[[asset]]
# File name containing transactions in this asset
# in CSV file.
filename = "transactions-EUR.csv"
# Name of the asset. Used as an identifier.
name = "EUR"
# The column in the CSV file containing the date of the transaction.
date_column = 0
# Format of the date field. %Y means zero-padded year, %m is zero-padded month,
# %d means zero-padded day of the month.
date_format = "%d.%m.%Y"
# Column containing the amount transacted. Positive amount means income,
# negative value means spending
amount_column = 4
# Separator used in CVS file. Must contain a single ASCII character.
# Defaults to ","
separator = ";"

[[asset]]
filename = "transactions-BTC.csv"
name = "BTC"
date_column = 1
date_format = "%Y-%m-%dT%H:%M:%S"
amount_column = 5

# Describes CSV file of price data
[[pair]]
# File name containing price data
filename = "kraken-EUR.csv"
# Currency in which the price is accounted
# The identifier must be the same as one of the asset identifiers!
accounting_currency = "EUR"
# The currency which is being priced.
# The identifier must be the same as one of the asset identifiers!
price_of = "BTC"
# The column in CSV file containing price
price_column = 1
# The column in CSV file containing date of the price change
date_column = 0
# The format of the date used.
date_format = "%Y%m%d"
```

Run with `cargo run CONFIG UNIT\_OF\_ACOUNT`. If you plan to use it often or for large data sets, run it
with `cargo run --release`. (Optimizations are negligible for small datasets.)

So if your config file is called "config.toml" and you want to measure your savings in EUR, run the command
`cargo run config.toml EUR`.

This will however not draw any plot, but just processes the data and spits out few files. You have to use
gnuplot to actually plot it.

The easiest way to do that is to run `gnuplot` (it will start interactive mode) and then load prepared script:

```
gnuplot> load "./plot_data.gnuplot"
```

This will open a window with the plot and allow you to inspect it or resize as you need.

Of course, if you have more or different assets, you have to modify the gnuplot script.

Understanding the plots
-----------------------

The plot shows several things at once:

* Grey line shows your total savings in selected unit of account.
* Blue line shows EUR portion of your savings.
* Brown line shows BTC portion of your savings.
* Magenta line shows you how much you would have if you converted all your money to BTC at the moment of
  receiving and held it as BTC.
* Cyan line shows you how much you would have if you converted all your money to EUR at the moment of
  receiving and held it as EUR.
* Green bars show income
* Red bars show spendings

Caveats
-------

This program works well for me but has some rough edges. Mainly:

* The program requires particular properties of your CSV files: UTF-8 encoding and being sorted by date from
  oldest to newest. This ensures good performance.
* The program doesn't attempt to chain pairs, so if you have e.g. EUR, BTC and XMR, want to account in EUR,
  then for it to work, you need EUR/BTC and EUR/XMR pairs. EUR/BTC and BTC/XMR wouldn't be sufficient. 
* The program uses floats to calculate balances, so some values will definitely be off.
* Some errors cause the program to panic instead of writing nicer error. This is to make debugging easier.
* The program is unable to correlate spending of one asset with receiving another asset of same value - when
  you trade. Fortunatelly, they cancel-out in reality, so the results are still useful. 
* Somewhat unrelated but keep in mind that fiat money is inflating, over time, so using more stable unit of
  account (e.g. gold) might give you better information.
