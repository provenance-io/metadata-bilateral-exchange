package io.provenance.bilateral.execute

import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.interfaces.ContractExecuteMsg

/*
    {
      "execute_match" : {
        "ask_id" : "fe3f6eaf-885f-4ea1-a2fe-a80e2fa745cd",
        "bid_id" : "c52eeda2-3224-4615-b5f9-e26a4a2f60a6",
        "accept_mismatched_bids": true
      }
    }

    With Funds: [ ]
 */
/**
 * An execute match contract route call must be made by the contract admin address.
 */
@JsonNaming(SnakeCaseStrategy::class)
data class ExecuteMatch(val executeMatch: Body) : ContractExecuteMsg {
    /**
     * @param askId The unique identifier of the ask to match with.
     * @param bidId The unique identifier of the bid to match with.
     * @param acceptMismatchedBids If true, a match will be executed even if the bid offers a lower (or even completely
     * different denom) coin than was requested in the ask's quote.  Ex: Asker requests 200nhash and bidder offers
     * 100nhash - refused unless this flag is 'true'.  Ex: Asker requests 200nhash and bidder offers 500000dogecoin -
     * refused unless this flag is 'true'.
     */
    @JsonNaming(SnakeCaseStrategy::class)
    data class Body(val askId: String, val bidId: String, val acceptMismatchedBids: Boolean?)

    companion object {
        fun new(
            askId: String,
            bidId: String,
            // The contract by default will use 'false' for this value if null is provided
            acceptMismatchedBids: Boolean? = null,
        ): ExecuteMatch = ExecuteMatch(
            executeMatch = Body(
                askId = askId,
                bidId = bidId,
                acceptMismatchedBids = acceptMismatchedBids,
            )
        )
    }
}
