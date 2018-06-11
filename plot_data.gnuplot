set title 'Balance history'
set xlabel 'Date'
set xdata time
set timefmt '%Y-%m-%d'
set style line 11 linecolor rgb "green"
set style line 12 linecolor rgb "red"

plot 'balances.txt' using 1:2 with lines lt rgb "grey" title "Total balance", \
 'balances-EUR.txt' using 1:2 with lines lt rgb "blue" title "EUR balance", \
 'balances-BTC.txt' using 1:2 with lines lt rgb "brown" title "BTC balance", \
 'maximalist-balances-EUR.txt' using 1:2 with lines lt rgb "cyan" title "Maximalist EUR balance", \
 'maximalist-balances-BTC.txt' using 1:2 with lines lt rgb "magenta" title "Maximalist BTC balance", \
 'transactions.txt' using 1:2:($2 < 0 ? 12 : 11) linecolor variable with impulses notitle
