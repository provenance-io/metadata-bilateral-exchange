package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.interfaces.ContractExecuteMsg

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("update_settings")
data class UpdateSettings(val update: Body) : ContractExecuteMsg {
    /**
     * The request contents for updating contract settings.
     *
     * @param newAdminAddress A new admin for which to execute matches and perform other actions.  If set to null, the
     * existing admin will not be changed.
     * @param askFee A fee to charge when new asks are created.  If set to null, no fee will be charged for future asks.
     * @param bidFee A fee to charge when new bids are created.  If set to null, no fee will be charged for future bids.
     */
    @JsonNaming(SnakeCaseStrategy::class)
    data class Body(
        val newAdminAddress: String?,
        val askFee: List<Coin>?,
        val bidFee: List<Coin>?,
    )

    companion object {
        fun new(
            newAdminAddress: String? = null,
            askFee: List<Coin>? = null,
            bidFee: List<Coin>? = null,
        ): UpdateSettings = UpdateSettings(
            update = Body(newAdminAddress, askFee, bidFee),
        )
    }
}
