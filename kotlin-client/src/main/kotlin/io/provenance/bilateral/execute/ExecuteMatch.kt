package io.provenance.bilateral.execute

import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.interfaces.ContractExecuteMsg

/**
 * An execute match call must be executed by the asker or the admin address.
 *
 * @param askId The unique identifier of the ask to match with.
 * @param bidId The unique identifier of the bid to match with.
 * @param acceptMismatchedBids If true, a match will be executed even if the bid offers a lower (or even completely
 * different denom) coin than was requested in the ask's quote.  Ex: Asker requests 200nhash and bidder offers
 * 100nhash - refused unless this flag is 'true'.  Ex: Asker requests 200nhash and bidder offers 500000dogecoin -
 * refused unless this flag is 'true'.
 */
@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("execute_match")
data class ExecuteMatch(
    val askId: String,
    val bidId: String,
    val acceptMismatchedBids: Boolean? = null,
) : ContractExecuteMsg {
    override fun toLoggingString(): String = "executeMatch, askId = [$askId], bidId = [$bidId], acceptMismatchedBids = [$acceptMismatchedBids]"
}
