package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.interfaces.ContractExecuteMsg
import io.provenance.bilateral.models.RequestDescriptor

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("update_bid")
data class UpdateBid(
    val bid: Bid,
    val descriptor: RequestDescriptor? = null,
) : ContractExecuteMsg {
    override fun toLoggingString(): String = "updateBid, ${bid.toLoggingStringSuffix()}"
}
