package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.interfaces.ContractExecuteMsg

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("cancel_ask")
data class CancelAsk(val id: String) : ContractExecuteMsg {
    override fun toLoggingString(): String = "cancelAsk, id = [$id]"
}
