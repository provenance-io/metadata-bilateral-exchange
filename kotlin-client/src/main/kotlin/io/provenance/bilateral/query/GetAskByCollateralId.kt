package io.provenance.bilateral.query

import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.interfaces.BilateralContractQueryMsg

/**
 * This request searches for an ask by its collateral id.  Each ask type's collateral id differs:
 * - Coin Trade: Identical to the ask id.
 * - Marker Trade: The marker's bech32 address (NOT denom).
 * - Marker Share Sale: The marker's bech32 address (NOT denom).
 * - Scope Trade: The scope's bech32 address.
 *
 * @param collateralId The unique collateral identifier for the target [io.provenance.bilateral.models.AskOrder].
 */
@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("get_ask_by_collateral_id")
data class GetAskByCollateralId(val collateralId: String) : BilateralContractQueryMsg {
    override fun toLoggingString(): String = "getAskByCollateralId, collateralId = [$collateralId]"
}
