package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies
import com.fasterxml.jackson.databind.annotation.JsonNaming
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.models.ShareSaleType
import io.provenance.bilateral.serialization.CosmWasmBigIntegerToUintSerializer
import io.provenance.bilateral.util.CoinUtil
import java.math.BigInteger

@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
sealed interface Ask {
    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
    @JsonTypeName("coin_trade")
    data class CoinTradeAsk(
        val id: String,
        val quote: List<Coin>,
        // This value is used as funds in the client and never included in the json payload
        @JsonIgnore
        val base: List<Coin>,
    ) : Ask

    /**
     * Note: A marker trade ask must be made AFTER the contract has been granted admin rights to the marker being
     * traded.
     */
    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
    @JsonTypeName("marker_trade")
    data class MarkerTradeAsk(
        val id: String,
        val markerDenom: String,
        val quotePerShare: List<Coin>,
    ) : Ask

    /**
     * Note: All marker share sales require that the contract be granted admin and withdraw rights on the marker
     * before the ask is created.  Recommended that this occurs in the same transaction.
     * Single share trades request that a specific number of shares be sold simultaneously in one bid match.
     * Multiple share trades allow any number of bids to be matched against the ask. The ask will only be deleted
     * in this circumstance once its shares have been depleted to zero (or if the share withdrawal limit has been
     * breached).
     */
    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
    @JsonTypeName("marker_share_sale")
    data class MarkerShareSaleAsk(
        val id: String,
        val markerDenom: String,
        @JsonSerialize(using = CosmWasmBigIntegerToUintSerializer::class)
        val sharesToSell: BigInteger,
        val quotePerShare: List<Coin>,
        val shareSaleType: ShareSaleType,
    ) : Ask

    @JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
    @JsonTypeName("scope_trade")
    data class ScopeTradeAsk(val id: String, val scopeAddress: String, val quote: List<Coin>) : Ask

    /**
     * Allows the ask type to be consumed and mapped based on value.  This can be used to derive an output type for any
     * of the request types.
     */
    fun <T> map(
        coinTrade: (coinTrade: CoinTradeAsk) -> T,
        markerTrade: (markerTrade: MarkerTradeAsk) -> T,
        markerShareSale: (markerShareSale: MarkerShareSaleAsk) -> T,
        scopeTrade: (scopeTrade: ScopeTradeAsk) -> T,
    ): T = when (this) {
        is CoinTradeAsk -> coinTrade(this)
        is MarkerTradeAsk -> markerTrade(this)
        is MarkerShareSaleAsk -> markerShareSale(this)
        is ScopeTradeAsk -> scopeTrade(this)
    }

    fun mapToId(): String = map(
        coinTrade = { coinTrade -> coinTrade.id },
        markerTrade = { markerTrade -> markerTrade.id },
        markerShareSale = { markerShareSale -> markerShareSale.id },
        scopeTrade = { scopeTrade -> scopeTrade.id },
    )

    fun mapToFunds(askFee: List<Coin>? = null): List<Coin> = map(
        coinTrade = { coinTrade -> coinTrade.base },
        markerTrade = { emptyList() },
        markerShareSale = { emptyList() },
        scopeTrade = { emptyList() },
    ).let { funds ->
        askFee?.let { CoinUtil.combineFunds(funds, it) } ?: funds
    }
}

/**
 * This is declared as an internal extension function to be used with both the CreateAsk and UpdateAsk toLoggingString
 * function overrides.  Interfaces do not allow internal function declarations, so this allows this functionality to
 * be private to the library without exposing unnecessary details to consumers.
 */
internal fun Ask.toLoggingStringSuffix(): String = this.map(
    coinTrade = { "askType = [coin_trade], id = [${it.id}]" },
    markerTrade = { "askType = [marker_trade], id = [${it.id}], markerDenom = [${it.markerDenom}]" },
    markerShareSale = { "askType = [marker_share_sale], id = [${it.id}], markerDenom = [${it.markerDenom}], sharesToSell = [${it.sharesToSell}]" },
    scopeTrade = { "askType = [scope_trade], id = [${it.id}], scopeAddress = [${it.scopeAddress}]" },
)
