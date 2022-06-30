# README

This repository houses a Provenance Blockchain Metadata bilateral exchange smart contract, and a client library that allows
interaction with it from a Kotlin application.  This project is based on the [Bilateral Exchange Smart Contract](https://github.com/provenance-io/bilateral-exchange).

## [Metadata Bilateral Exchange Smart Contract](./smart-contract)

The Metadata Bilateral Exchange smart contract is written in Rust, leveraging the [Cosmwasm SDK](https://crates.io/crates/cosmwasm-std)
and [Provwasm SDK](https://crates.io/crates/provwasm-std) to produce a WASM file compatible with the [Provenance Blockchain](https://github.com/provenance-io/provenance).
This contract is designed to allow the bilateral exchange of [Provenance Blockchain Metadata Module](https://docs.provenance.io/modules/metadata-module) 
and [Provenance Blockchain Marker Module](https://docs.provenance.io/modules/marker-module) goods for [coin](https://docs.provenance.io/blockchain/basics/stablecoin).

## [Kotlin Client](./kotlin-client)

The Kotlin client is written in Kotlin, leveraging the [PbClient](https://github.com/provenance-io/pb-grpc-client-kotlin)
to facilitate requests to a [Provenance Blockchain](https://github.com/provenance-io/provenance) instance of the
[Metadata Bilateral Exchange Smart Contract](./smart-contract).
