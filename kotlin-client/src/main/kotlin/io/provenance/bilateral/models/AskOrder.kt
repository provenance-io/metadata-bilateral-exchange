package io.provenance.bilateral.models

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.models.AskCollateral.CoinTrade
import io.provenance.bilateral.models.AskCollateral.MarkerShareSale
import io.provenance.bilateral.models.AskCollateral.MarkerTrade
import io.provenance.bilateral.models.AskCollateral.ScopeTrade

@JsonNaming(SnakeCaseStrategy::class)
data class AskOrder(
    val id: String,
    val askType: String,
    val owner: String,
    val collateral: AskCollateral,
    val descriptor: RequestDescriptor?
) {
    @JsonIgnore
    fun <T> mapCollateral(
        coinTrade: (coinTrade: CoinTrade) -> T,
        markerTrade: (markerTrade: MarkerTrade) -> T,
        markerShareSale: (markerShareSale: MarkerShareSale) -> T,
        scopeTrade: (scopeTrade: ScopeTrade) -> T,
    ): T = when (collateral) {
        is CoinTrade -> coinTrade(collateral)
        is MarkerTrade -> markerTrade(collateral)
        is MarkerShareSale -> markerShareSale(collateral)
        is ScopeTrade -> scopeTrade(collateral)
    }
}

@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
sealed interface AskCollateral {
    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("coin_trade")
    data class CoinTrade(
        val base: List<Coin>,
        val quote: List<Coin>,
    ) : AskCollateral

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("marker_trade")
    data class MarkerTrade(
        val markerAddress: String,
        val markerDenom: String,
        val quotePerShare: List<Coin>,
        val removedPermissions: List<MarkerAccessGrant>,
    ) : AskCollateral

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("marker_share_sale")
    data class MarkerShareSale(
        val markerAddress: String,
        val markerDenom: String,
        val totalSharesInSale: String,
        val remainingSharesInSale: String,
        val quotePerShare: List<Coin>,
        val removedPermissions: List<MarkerAccessGrant>,
        val saleType: ShareSaleType,
    ) : AskCollateral

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("scope_trade")
    data class ScopeTrade(
        val scopeAddress: String,
        val quote: List<Coin>,
    ) : AskCollateral
}

data class MarkerAccessGrant(
    val address: String,
    val permissions: List<String>,
)
