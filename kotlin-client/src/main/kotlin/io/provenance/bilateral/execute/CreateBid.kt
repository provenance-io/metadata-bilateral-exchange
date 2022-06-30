package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.execute.Bid.CoinTradeBid
import io.provenance.bilateral.execute.Bid.MarkerShareSaleBid
import io.provenance.bilateral.execute.Bid.MarkerTradeBid
import io.provenance.bilateral.execute.Bid.ScopeTradeBid
import io.provenance.bilateral.interfaces.ContractExecuteMsg
import io.provenance.bilateral.models.RequestDescriptor

@JsonNaming(SnakeCaseStrategy::class)
data class CreateBid(val createBid: Body) : ContractExecuteMsg {
    @JsonNaming(SnakeCaseStrategy::class)
    data class Body(val bid: Bid, val descriptor: RequestDescriptor?)

    companion object {
        /*
                {
                  "create_bid" : {
                    "bid" : {
                      "coin_trade" : {
                        "id" : "c52eeda2-3224-4615-b5f9-e26a4a2f60a6",
                        "base" : [ {
                          "denom" : "nhash",
                          "amount" : "50"
                        } ]
                      }
                    },
                    "descriptor" : {
                      "description" : "Example description",
                      "effective_time" : "1655690324377129000",
                      "attribute_requirement" : {
                         "attributes" : [ "something.pb" ],
                         "requirement_type": "any"
                      }
                    }
                  }
                }

                // Derived from QUOTE
                With Funds: [ {
                  "denom" : "nhash",
                  "amount" : "100"
                } ]
         */
        fun newCoinTrade(
            id: String,
            quote: List<Coin>,
            base: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            createBid = Body(
                bid = Bid.newCoinTrade(
                    id = id,
                    quote = quote,
                    base = base,
                ),
                descriptor = descriptor,
            )
        )

        /*
            {
              "create_bid" : {
                "bid" : {
                  "marker_trade" : {
                    "id" : "d186dd8d-5068-4b62-a118-d33fcb2cd544",
                    "denom" : "testcoin"
                  }
                },
                "descriptor" : {
                  "description" : "Example description",
                  "effective_time" : "1655690835272007000",
                  "attribute_requirement": {
                    "attributes" : [ "attribute.pb", "otherattribute.pb" ],
                    "requirement_type" : "all"
                  }
                }
              }
            }

            With Funds: [ {
              "denom" : "nhash",
              "amount" : "500"
            } ]
         */
        fun newMarkerTrade(
            id: String,
            denom: String,
            quote: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            createBid = Body(
                bid = Bid.newMarkerTrade(
                    id = id,
                    denom = denom,
                    quote = quote,
                ),
                descriptor = descriptor,
            )
        )

        /*
            SINGLE TRANSACTION TRADE:

            {
              "create_bid" : {
                "bid" : {
                  "marker_share_sale" : {
                    "id" : "ee44d587-fd11-4803-b372-a820c41c4dfa",
                    "denom" : "dankcoin",
                    "share_count" : "75"
                  }
                },
                "descriptor" : {
                  "description" : "Example description",
                  "effective_time" : "1655691531898653000",
                  "attribute_requirement" : {
                    "attributes": [ "badattribute.pio" ],
                    "requirement_type" : "none"
                }
              }
            }

            With Funds: [ {
              "denom" : "nhash",
              "amount" : "3750"
            } ]
         */

        /*
            MULTIPLE TRANSACTION TRADE:

            {
              "create_bid" : {
                "bid" : {
                  "marker_share_sale" : {
                    "id" : "943b7f98-ffcd-4174-99a4-fda94f6a8f7c",
                    "denom" : "noucoin",
                    "share_count" : "25"
                  }
                },
                "descriptor" : {
                  "description" : "Example description",
                  "effective_time" : "1655691962780823000",
                  "attribute_requirement" : {
                    "attributes" : [ "a.pb", "b.pio" ],
                    "requirement_type" : "all"
                  }
                }
              }
            }

            With Funds: [ {
              "denom" : "nhash",
              "amount" : "25000"
            } ]
         */
        fun newMarkerShareSale(
            id: String,
            denom: String,
            shareCount: String,
            quote: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            createBid = Body(
                bid = Bid.newMarkerShareSale(
                    id = id,
                    denom = denom,
                    shareCount = shareCount,
                    quote = quote,
                ),
                descriptor = descriptor,
            )
        )

        /*
            {
              "create_bid" : {
                "bid" : {
                  "scope_trade" : {
                    "id" : "721305c5-4a82-4174-81ed-225342f9e377",
                    "scope_address" : "scope1qz9puy0kqex5xfawzunfqrw25htquqr5ns"
                  }
                },
                "descriptor" : {
                  "description" : "Example description",
                  "effective_time" : "1655692123071177000",
                  "attribute_requirement" : {
                    "attributes" : [ "attr.sc.pb", "other.pio" ],
                    "requirement_type" : "any"
                  }
                }
              }
            }
            With Funds: [ {
              "denom" : "nhash",
              "amount" : "50000"
            } ]
         */
        fun newScopeTrade(
            id: String,
            scopeAddress: String,
            quote: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            createBid = Body(
                bid = Bid.newScopeTrade(
                    id = id,
                    scopeAddress = scopeAddress,
                    quote = quote,
                ),
                descriptor = descriptor,
            )
        )
    }

    @JsonIgnore
    fun <T> mapBid(
        coinTrade: (coinTrade: CoinTradeBid.Body) -> T,
        markerTrade: (markerTrade: MarkerTradeBid.Body) -> T,
        markerShareSale: (markerShareSale: MarkerShareSaleBid.Body) -> T,
        scopeTrade: (scopeTrade: ScopeTradeBid.Body) -> T,
    ): T = when (this.createBid.bid) {
        is CoinTradeBid -> coinTrade(this.createBid.bid.coinTrade)
        is MarkerTradeBid -> markerTrade(this.createBid.bid.markerTrade)
        is MarkerShareSaleBid -> markerShareSale(this.createBid.bid.markerShareSale)
        is ScopeTradeBid -> scopeTrade(this.createBid.bid.scopeTrade)
    }

    @JsonIgnore
    fun getId(): String = mapBid(
        coinTrade = { coinTrade -> coinTrade.id },
        markerTrade = { markerTrade -> markerTrade.id },
        markerShareSale = { markerShareSale -> markerShareSale.id },
        scopeTrade = { scopeTrade -> scopeTrade.id },
    )

    @JsonIgnore
    fun getFunds(): List<Coin> = mapBid(
        coinTrade = { coinTrade -> coinTrade.quote },
        markerTrade = { markerTrade -> markerTrade.quote },
        markerShareSale = { markerShareSale -> markerShareSale.quote },
        scopeTrade = { scopeTrade -> scopeTrade.quote },
    )
}

sealed interface Bid {
    @JsonNaming(SnakeCaseStrategy::class)
    data class CoinTradeBid(val coinTrade: Body) : Bid {
        @JsonNaming(SnakeCaseStrategy::class)
        data class Body(
            val id: String,
            // The quote is used for funds, and never added to the json payload send to the contract
            @JsonIgnore
            val quote: List<Coin>,
            val base: List<Coin>,
        )
    }

    @JsonNaming(SnakeCaseStrategy::class)
    data class MarkerTradeBid(val markerTrade: Body) : Bid {
        @JsonNaming(SnakeCaseStrategy::class)
        data class Body(
            val id: String,
            val denom: String,
            // The quote is used for funds, and never added to the json payload send to the contract
            @JsonIgnore
            val quote: List<Coin>,
        )
    }

    @JsonNaming(SnakeCaseStrategy::class)
    data class MarkerShareSaleBid(val markerShareSale: Body) : Bid {
        @JsonNaming(SnakeCaseStrategy::class)
        data class Body(
            val id: String,
            val denom: String,
            val shareCount: String,
            // The quote is used for funds, and never added to the json payload send to the contract
            @JsonIgnore
            val quote: List<Coin>,
        )
    }

    @JsonNaming(SnakeCaseStrategy::class)
    data class ScopeTradeBid(val scopeTrade: Body) : Bid {
        @JsonNaming(SnakeCaseStrategy::class)
        data class Body(
            val id: String,
            val scopeAddress: String,
            // The quote is used for funds, and never added to the json payload send to the contract
            @JsonIgnore
            val quote: List<Coin>,
        )
    }

    companion object {
        fun newCoinTrade(id: String, quote: List<Coin>, base: List<Coin>): Bid = CoinTradeBid(
            coinTrade = CoinTradeBid.Body(id, quote, base)
        )

        fun newMarkerTrade(id: String, denom: String, quote: List<Coin>): Bid = MarkerTradeBid(
            markerTrade = MarkerTradeBid.Body(id, denom, quote)
        )

        fun newMarkerShareSale(id: String, denom: String, shareCount: String, quote: List<Coin>): Bid = MarkerShareSaleBid(
            markerShareSale = MarkerShareSaleBid.Body(id, denom, shareCount, quote)
        )

        fun newScopeTrade(id: String, scopeAddress: String, quote: List<Coin>): Bid = ScopeTradeBid(
            scopeTrade = ScopeTradeBid.Body(id, scopeAddress, quote)
        )
    }
}
