rm -rf ./_cache
../target/release/ord --log-level=INFO --data-dir=./_cache --index-sats --rpc-url=<url> --regtest --bitcoin-rpc-user <user> --bitcoin-rpc-pass <password> --brczero-rpc-url <url> --first-brczero-height 1500 server