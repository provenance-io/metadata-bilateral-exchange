package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies
import com.fasterxml.jackson.databind.annotation.JsonNaming
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.serialization.CosmWasmBigIntegerToUintSerializer
import io.provenance.bilateral.util.CoinUtil
import java.math.BigInteger

@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
sealed interface Bid {
    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
    @JsonTypeName("coin_trade")
    data class CoinTradeBid(
        val id: String,
        // The quote is used for funds, and never added to the json payload send to the contract
        @JsonIgnore
        val quote: List<Coin>,
        val base: List<Coin>,
    ) : Bid

    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
    @JsonTypeName("marker_trade")
    data class MarkerTradeBid(
        val id: String,
        val markerDenom: String,
        // The quote is used for funds, and never added to the json payload send to the contract
        @JsonIgnore
        val quote: List<Coin>,
    ) : Bid

    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
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

    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
    @JsonTypeName("scope_trade")
    data class ScopeTradeBid(
        val id: String,
        val scopeAddress: String,
        // The quote is used for funds, and never added to the json payload send to the contract
        @JsonIgnore
        val quote: List<Coin>,
    ) : Bid

    fun <T> map(
        coinTrade: (coinTrade: CoinTradeBid) -> T,
        markerTrade: (markerTrade: MarkerTradeBid) -> T,
        markerShareSale: (markerShareSale: MarkerShareSaleBid) -> T,
        scopeTrade: (scopeTrade: ScopeTradeBid) -> T,
    ): T = when (this) {
        is CoinTradeBid -> coinTrade(this)
        is MarkerTradeBid -> markerTrade(this)
        is MarkerShareSaleBid -> markerShareSale(this)
        is ScopeTradeBid -> scopeTrade(this)
    }

    fun mapToId(): String = map(
        coinTrade = { coinTrade -> coinTrade.id },
        markerTrade = { markerTrade -> markerTrade.id },
        markerShareSale = { markerShareSale -> markerShareSale.id },
        scopeTrade = { scopeTrade -> scopeTrade.id },
    )

    fun mapToFunds(bidFee: List<Coin>? = null): List<Coin> = map(
        coinTrade = { coinTrade -> coinTrade.quote },
        markerTrade = { markerTrade -> markerTrade.quote },
        markerShareSale = { markerShareSale -> markerShareSale.quote },
        scopeTrade = { scopeTrade -> scopeTrade.quote },
    ).let { funds ->
        bidFee?.let { CoinUtil.combineFunds(funds, bidFee) } ?: funds
    }
}

/**
 * This is declared as an internal extension function to be used with both the CreateBid and UpdateBid toLoggingString
 * function overrides.  Interfaces do not allow internal function declarations, so this allows this functionality to
 * be private to the library without exposing unnecessary details to consumers.
 */
internal fun Bid.toLoggingStringSuffix(): String = this.map(
    coinTrade = { "bidType = [coin_trade], id = [${it.id}]" },
    markerTrade = { "bidType = [marker_trade], id = [${it.id}], markerDenom = [${it.markerDenom}]" },
    markerShareSale = { "bidType = [marker_share_sale], id = [${it.id}], markerDenom = [${it.markerDenom}], shareCount = [${it.shareCount}]" },
    scopeTrade = { "bidType = [scope_trade], id = [${it.id}], scopeAddress = [${it.scopeAddress}]" },
)
