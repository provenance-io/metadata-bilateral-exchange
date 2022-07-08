package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.execute.Bid.CoinTradeBid
import io.provenance.bilateral.execute.Bid.MarkerShareSaleBid
import io.provenance.bilateral.execute.Bid.MarkerTradeBid
import io.provenance.bilateral.execute.Bid.ScopeTradeBid
import io.provenance.bilateral.interfaces.ContractExecuteMsg
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.bilateral.serialization.CosmWasmBigIntegerToUintSerializer
import io.provenance.bilateral.util.CoinUtil
import java.math.BigInteger

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("create_bid")
data class CreateBid(val bid: Bid, val descriptor: RequestDescriptor?) : ContractExecuteMsg {
    companion object {
        fun newCoinTrade(
            id: String,
            quote: List<Coin>,
            base: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            bid = CoinTradeBid(
                id = id,
                quote = quote,
                base = base,
            ),
            descriptor = descriptor,
        )

        fun newMarkerTrade(
            id: String,
            markerDenom: String,
            quote: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            bid = MarkerTradeBid(
                id = id,
                markerDenom = markerDenom,
                quote = quote,
            ),
            descriptor = descriptor,
        )

        fun newMarkerShareSale(
            id: String,
            markerDenom: String,
            shareCount: BigInteger,
            quote: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            bid = MarkerShareSaleBid(
                id = id,
                markerDenom = markerDenom,
                shareCount = shareCount,
                quote = quote,
            ),
            descriptor = descriptor,
        )

        fun newScopeTrade(
            id: String,
            scopeAddress: String,
            quote: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateBid = CreateBid(
            bid = ScopeTradeBid(
                id = id,
                scopeAddress = scopeAddress,
                quote = quote,
            ),
            descriptor = descriptor,
        )
    }

    @JsonIgnore
    fun <T> mapBid(
        coinTrade: (coinTrade: CoinTradeBid) -> T,
        markerTrade: (markerTrade: MarkerTradeBid) -> T,
        markerShareSale: (markerShareSale: MarkerShareSaleBid) -> T,
        scopeTrade: (scopeTrade: ScopeTradeBid) -> T,
    ): T = when (bid) {
        is CoinTradeBid -> coinTrade(bid)
        is MarkerTradeBid -> markerTrade(bid)
        is MarkerShareSaleBid -> markerShareSale(bid)
        is ScopeTradeBid -> scopeTrade(bid)
    }

    @JsonIgnore
    fun getId(): String = mapBid(
        coinTrade = { coinTrade -> coinTrade.id },
        markerTrade = { markerTrade -> markerTrade.id },
        markerShareSale = { markerShareSale -> markerShareSale.id },
        scopeTrade = { scopeTrade -> scopeTrade.id },
    )

    @JsonIgnore
    fun getFunds(bidFee: List<Coin>?): List<Coin> = mapBid(
        coinTrade = { coinTrade -> coinTrade.quote },
        markerTrade = { markerTrade -> markerTrade.quote },
        markerShareSale = { markerShareSale -> markerShareSale.quote },
        scopeTrade = { scopeTrade -> scopeTrade.quote },
    ).let { funds ->
        bidFee?.let { CoinUtil.combineFunds(funds, bidFee) } ?: funds
    }
}

@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
sealed interface Bid {
    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("coin_trade")
    data class CoinTradeBid(
        val id: String,
        // The quote is used for funds, and never added to the json payload send to the contract
        @JsonIgnore
        val quote: List<Coin>,
        val base: List<Coin>,
    ) : Bid

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("marker_trade")
    data class MarkerTradeBid(
        val id: String,
        val markerDenom: String,
        // The quote is used for funds, and never added to the json payload send to the contract
        @JsonIgnore
        val quote: List<Coin>,
    ) : Bid

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("marker_share_sale")
    data class MarkerShareSaleBid(
        val id: String,
        val markerDenom: String,
        @JsonSerialize(using = CosmWasmBigIntegerToUintSerializer::class)
        val shareCount: BigInteger,
        // The quote is used for funds, and never added to the json payload send to the contract
        @JsonIgnore
        val quote: List<Coin>,
    ) : Bid

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("scope_trade")
    data class ScopeTradeBid(
        val id: String,
        val scopeAddress: String,
        // The quote is used for funds, and never added to the json payload send to the contract
        @JsonIgnore
        val quote: List<Coin>,
    ) : Bid
}
