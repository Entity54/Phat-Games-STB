# Hello World in Phat Contract

<img align="right" width="320" src="res/Phat%20Contract_Standard%20Logo_wht_02.svg">

This contract shows the off-chain computation with native HTTP request support in Phat Contract from [Phala Network](https://phala.network/).

It receives the Ethereum accounts from users and reports the account balance by querying the Etherscan with its native HTTP request.

## Build

Setup the environment for Ink! contract compilation, then run

```bash
cargo +nightly contract build
```

## Test

To test your contract locally and see its output, run with

```bash
cargo +nightly test -- --nocapture
```
