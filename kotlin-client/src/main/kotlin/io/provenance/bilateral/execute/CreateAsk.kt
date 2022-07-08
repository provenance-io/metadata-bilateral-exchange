package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.execute.Ask.CoinTradeAsk
import io.provenance.bilateral.execute.Ask.MarkerShareSaleAsk
import io.provenance.bilateral.execute.Ask.MarkerTradeAsk
import io.provenance.bilateral.execute.Ask.ScopeTradeAsk
import io.provenance.bilateral.interfaces.ContractExecuteMsg
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.bilateral.models.ShareSaleType
import io.provenance.bilateral.util.CoinUtil

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("create_ask")
data class CreateAsk(
    val ask: Ask,
    val descriptor: RequestDescriptor? = null,
) : ContractExecuteMsg {
    companion object {
        fun newCoinTrade(
            id: String,
            quote: List<Coin>,
            base: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateAsk = CreateAsk(
            ask = CoinTradeAsk(id, quote, base),
            descriptor = descriptor,
        )

        /**
         * Note: A marker trade ask must be made AFTER the contract has been granted admin rights to the marker being
         * traded.
         */
        fun newMarkerTrade(
            id: String,
            markerDenom: String,
            quotePerShare: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateAsk = CreateAsk(
            ask = MarkerTradeAsk(id, markerDenom, quotePerShare),
            descriptor = descriptor,
        )

        /**
         * Note: All marker share sales require that the contract be granted admin and withdraw rights on the marker
         * before the ask is created.  Recommended that this occurs in the same transaction.
         * Single share trades request that a specific number of shares be sold simultaneously in one bid match.
         * Multiple share trades allow any number of bids to be matched against the ask. The ask will only be deleted
         * in this circumstance once its shares have been depleted to zero (or if the share withdrawal limit has been
         * breached).
         */
        fun newMarkerShareSale(
            id: String,
            markerDenom: String,
            sharesToSell: String,
            quotePerShare: List<Coin>,
            shareSaleType: ShareSaleType,
            descriptor: RequestDescriptor? = null,
        ): CreateAsk = CreateAsk(
            ask = MarkerShareSaleAsk(id, markerDenom, sharesToSell, quotePerShare, shareSaleType),
            descriptor = descriptor,
        )

        fun newScopeTrade(
            id: String,
            scopeAddress: String,
            quote: List<Coin>,
            descriptor: RequestDescriptor? = null,
        ): CreateAsk = CreateAsk(
            ask = ScopeTradeAsk(id, scopeAddress, quote),
            descriptor = descriptor,
        )
    }

    /**
     * Allows the ask type to be consumed and mapped based on value.  This can be used to derive an output type for any
     * of the request types.
     */
    @JsonIgnore
    fun <T> mapAsk(
        coinTrade: (coinTrade: CoinTradeAsk) -> T,
        markerTrade: (markerTrade: MarkerTradeAsk) -> T,
        markerShareSale: (markerShareSale: MarkerShareSaleAsk) -> T,
        scopeTrade: (scopeTrade: ScopeTradeAsk) -> T,
    ): T = when (ask) {
        is CoinTradeAsk -> coinTrade(ask)
        is MarkerTradeAsk -> markerTrade(ask)
        is MarkerShareSaleAsk -> markerShareSale(ask)
        is ScopeTradeAsk -> scopeTrade(ask)
    }

    @JsonIgnore
    fun getId(): String = mapAsk(
        coinTrade = { coinTrade -> coinTrade.id },
        markerTrade = { markerTrade -> markerTrade.id },
        markerShareSale = { markerShareSale -> markerShareSale.id },
        scopeTrade = { scopeTrade -> scopeTrade.id },
    )

    @JsonIgnore
    fun getFunds(askFee: List<Coin>?): List<Coin> = mapAsk(
        coinTrade = { coinTrade -> coinTrade.base },
        markerTrade = { emptyList() },
        markerShareSale = { emptyList() },
        scopeTrade = { emptyList() },
    ).let { funds ->
        askFee?.let { CoinUtil.combineFunds(funds, it) } ?: funds
    }
}

@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
sealed interface Ask {
    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("coin_trade")
    data class CoinTradeAsk(
        val id: String,
        val quote: List<Coin>,
        // This value is used as funds in the client and never included in the json payload
        @JsonIgnore
        val base: List<Coin>,
    ) : Ask

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("marker_trade")
    data class MarkerTradeAsk(
        val id: String,
        val markerDenom: String,
        val quotePerShare: List<Coin>,
    ) : Ask

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("marker_share_sale")
    data class MarkerShareSaleAsk(
        val id: String,
        val markerDenom: String,
        val sharesToSell: String,
        val quotePerShare: List<Coin>,
        val shareSaleType: ShareSaleType,
    ) : Ask

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("scope_trade")
    data class ScopeTradeAsk(val id: String, val scopeAddress: String, val quote: List<Coin>) : Ask
}
