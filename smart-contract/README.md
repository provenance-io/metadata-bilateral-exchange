# Metadata Bilateral Exchange Smart Contract

This a [CosmWasm](https://crates.io/crates/cosmwasm-std) smart contract that provides the bilateral exchange of 
[Provenance Blockchain](https://docs.provenance.io) [markers](https://docs.provenance.io/modules/marker-module),
[coin](https://docs.provenance.io/blockchain/basics/stablecoin) and [scopes](https://docs.provenance.io/modules/metadata-module#scope-data-structures).

## Status
[![Latest Release][release-badge]][release-latest]
[![Apache 2.0 License][license-badge]][license-url]

[release-badge]: https://img.shields.io/github/tag/provenance-io/metadata-bilateral-exchange.svg
[release-latest]: https://github.com/provenance-io/metadata-bilateral-exchange/releases/latest
[license-badge]: https://img.shields.io/github/license/provenance-io/metadata-bilateral-exchange.svg
[license-url]: https://github.com/provenance-io/metadata-bilateral-exchange/blob/main/LICENSE

## Functions and Terminology

The contract functions by allowing two parties to exchange owned goods on the blockchain with a three step process:
ask, bid, and match.

### Ask
An ask is when an account invokes the `create_ask` execution route of the contract, stating that they will offer their
goods (marker, scope, or coin - a.k.a. "the base") for another account's coin (a.k.a. "the quote").  When an ask is placed,
the base is held by the contract until a match or a cancellation occurs.

### Bid
A bid is when an account invokes the `create_bid` execution route of the contract, stating that they will offer coin (a.k.a. "the quote")
for an asker's goods (marker, scope, or coin - a.k.a. "the base").  When the bid is placed, the quote funds are held by
the contract until a match or a cancellation occurs.

### Match
A match is when an ask and a bid are considered to be valid for each other, and the exchange of goods commences.  A match
can only occur when the bidder has placed the appropriate quote that matches the asker's specifications.  Once the match
is completed, the asker will receive the requested quote funds in coin, and the bidder will receive the goods specified
by the asker.  Matches must be executed by the contract's admin account.

### Cancellation
At any time before a match occurs, an asker or bidder may cancel their ask or bid order.  When this occurs, any goods
held by the contract on the behalf of the asker or bidder will be returned to the originating account in totality.

### Collateral
During each placement of ask or bid, a coin, scope, or marker will be held by the contract using various means of 
ensuring that the contract is the sole owner of the object until a match or cancellation occurs.  The held values are 
stored in the contract's internal storage in the form of an `AskOrder` or `BidOrder`.  These values can be searched for
and queried at any time to ensure full transparency of the held goods by the contract.

### Trade Types
The contract allows for four types of trade.  In all trade types, the bidder sends coin as the quote in exchange for
an asker's goods.

#### Coin Trade
In this trade, the asker sends coin as the base.  When a match is made, the asker simply receives their
quote coin and the bidder receives their base coin, completing the trade.

#### Marker Trade
In this trade, the asker sends a marker as the base.  The contract's address must be given admin 
permissions on the marker prior to the asker invoking the `create_ask` execution route for this trade type to be accepted.
Additionally, only an administrator of a marker may list a marker for trade. At least one of the marker's coins must 
remain in the marker's account holdings, and the asker specifies how much should be paid per coin by the bidder.  The 
marker is then stripped of all permissions by the contract to ensure that no modifications can be made before a 
cancellation or match occurs.  On a match, the asker then receives their quote, and the bidder receives permissions on 
the marker equal to those that the asker possessed when they listed it.  On a cancellation, the asker just receives
their permissions again on the marker.  Note: The contract will automatically remove its own permissions on the marker
as soon as the match is completed or the ask is cancelled.

#### Marker Share Sale
In this trade, the asker lists an amount of marker-held denom as the base.  The contract's address must be given admin
AND withdraw permissions on the marker prior to the asker invoking the `create_ask` execution route for this trade type
to be accepted.  Additionally, only an administrator of the marker may list a marker for trade and at least one of the
marker's coins must remain in the marker's account holdings.  The marker is then stripped of all permissions by the
contract to ensure that no modifications can be made before a cancellation or match occurs.  The behavior of this listing
differs based on the share sale type:

* _Single Share Sale_: In this type, the asker chooses a `share_count`, which specifies how many shares of the marker
will be sold.  This number must, of course, be less than or equal to the remaining marker shares held by the marker
account.  The asker also specifies a `quote_per_share`, which indicates how much each share will be sold for. After a match occurs,
the asker is sent the quote funds from the bidder, and the asker's marker permissions are restored.  The bidder receives
as many shares as were listed as the `share_count` value, and the ask and bid orders are deleted from the contract.

_Example_: If the asker lists `10` as the `share_count` and `20nhash` as the `quote_per_share`, then the bidder must list a quote of
`10` purchased shares at `200nhash`.  This will produce a valid match and the trade can occur.

* _Multiple Share Sale_: In this type, the asker specifies a `remove_sale_share_threshold`, which indicates the amount
of shares remaining at the end of the sale.  If no value is provided, the contract assumes the value to be zero and will
not stop the sale until all marker shares have been sold to bidders.  Like the single share type, the asker also provides
a `quote_per_share` value, specifying how much each share costs to purchase.  Each bidder provides how many shares they
wish to purchase, as well as a properly calculated quote.  When a match is made, the asker receives the quote from the
bidder, and the bidder receives their specified count of shares.  The contract then determines if the marker has any
shares remaining above the `remove_sale_share_threshold`.  If it does, the ask remains available and more bids can be
matched against it.  If it does not, the ask is deleted and the marker permissions are restored to the asker.  In all
cases, the bid order is always deleted after the bidder receives their marker shares.  A match will never be accepted if
the bid intends to purchase enough shares to reduce the marker's share holdings beneath the `remove_sale_share_threshold`.

_Example_: The asker has a marker with 100 shares of `markerdenom`.  They list the `remove_sale_share_threshold` as `50` and the 
`quote_per_share` as `10nhash`.  A bidder produces a request for `15` shares with the proper quote of `150nhash`.  The
match is made, the asker receives `150nhash` the bidder receives `15markerdenom` and the bid is deleted.  The remaining
balance of shares is 85, so the ask remains in the contract and the marker is not yet returned to the asker.  A second
bid comes in for `35` shares at `350nhash`.  The match is made, the asker receives `350nhash` and the bidder receives
`35markerdenom`.  The ask and bid are deleted, and the asker receives their permissions to the marker again.

#### Scope Trade
In this trade, the asker lists a scope as the base, and a coin request as the quote.  The contract must be listed as the sole `owner` in the scope's
ownership array, and the contract must also be listed as the `value_owner_address`.  Due to this requirement, it is
highly recommended that the transfer of ownership from the asker to the contract on the scope be bundled into the same
transaction that creates the ask to ensure that the scope is not "trapped" as owned by the contract if the create ask
fails.  A scope would be "trapped" because the contract would own it, but not have any `AskOrder` record of it, preventing
the contract from returning the scope with a `cancel_ask` execution route invocation.

When a match is made, asker receives the quote coins, the bidder is assigned as the sole `owner` and `value_owner` of
the scope, and both ask and bids are deleted.

## Build and Deployment

### Build Contract:
To create a local `.wasm` file representation of the contract for deployment to the [Provenance Blockchain](https://provenance.io),
just run the following:
```bash
make optimize
```
The contract will then be output to a local `artifacts` directory.

### Store Contract:
After the `.wasm` file is built, it needs to be stored on the chain.  To do so, some variation of the following command
must be run with the `provenanced` tool provided by the [Provenance Repository](https://github.com/provenance-io/provenance):

```bash
provenanced tx wasm store ~/path/to/this-directory/artifacts/metadata_bilateral_exchange.wasm \
--instantiate-only-address "some-admin-account-bech32" \
--from same-admin-account-key \
--home key/home \
--chain-id chain-local \
--gas auto \
--gas-prices="1905nhash" \
--gas-adjustment=1.2 \
--broadcast-mode block \
--testnet \
--output json
```

On a success, keep a record of the `code_id` value produced by the transaction.  This will be used to instantiate
the contract.

_Keep in mind_:
* An `instantiate-only-address` is not required.  If this flag is given, only the address supplied will ever be able
to use the code instance created.
* The `from` flag refers to the directory that your account keys are stored within.  This can be any directory, and will
automatically be created when `provenanced keys add` command is used with a `home` flag.
* The `chain-id` flag in this example refers to a localnet.  This is the target environment.  For instance, deployments
to the standard provenance testnet environment will use `pio-testnet-1` here.

### Instantiate Contract:
With your stored contract's `code_id`, run the following command to instantiate this contract in the target environment:

```bash
provenanced tx wasm instantiate 169 \ 
  '{
   "bind_name": "mybindingname.pb",
   "contract_name": "Metadata Bilateral Exchange"
 }' \
  --admin "some-admin-account-bech32" \
  --from same-admin-account-key \
  --label bilateral \
  --chain-id chain-local \
  --gas auto \
  --gas-adjustment 1.1 \
  --home build/node0 \
  --broadcast-mode block \
  --testnet \
  --output json
```

On a success, a `contract_address` value will be produced, indicating that the contract has been successfully instantiated
on the chain and can be interacted with using that address.  

_Keep in mind_:
* The `bind_name` value in the json will bind a [Provenance Blockchain Name](https://docs.provenance.io/modules/name-module)
to the contract that can be used to resolve the contract's address.
* If the `instantiate-only-address` was used in the previous step, that same address must be used in this step as the
`admin` address.
