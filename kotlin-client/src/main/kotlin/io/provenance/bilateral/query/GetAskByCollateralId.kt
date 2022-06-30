package io.provenance.bilateral.query

import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.interfaces.ContractQueryMsg

/*
    {
        "get_ask_by_collateral_id": {
            "collateral_id": "tp1gy7q48cyj597tcarglvxjl3ce7l64t5egxpetq"
        }
    }
 */
/**
 * This request searches for an ask by its collateral id.  Each ask type's collateral id differs:
 * - Coin Trade: Identical to the ask id.
 * - Marker Trade: The marker's bech32 address (NOT denom).
 * - Marker Share Sale: The marker's bech32 address (NOT denom).
 * - Scope Trade: The scope's bech32 address.
 */
@JsonNaming(SnakeCaseStrategy::class)
data class GetAskByCollateralId(val getAskByCollateralId: Body) : ContractQueryMsg {
    @JsonNaming(SnakeCaseStrategy::class)
    data class Body(val collateralId: String)

    companion object {
        fun new(collateralId: String): GetAskByCollateralId = GetAskByCollateralId(Body(collateralId))
    }
}
