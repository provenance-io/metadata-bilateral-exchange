package io.provenance.bilateral.models.executeresponse

/**
 * The data response returned after a match between an ask and bid is successfully made.
 *
 * @param askId The unique identifier of the matched ask.
 * @param bidId The unique identifier of the matched bid.
 * @param askDeleted Whether or not the ask was deleted after the match was completed.  In most cases, the ask is
 * deleted when a match is made, because no more information about the ask is needed within the contract.
 * @param bidDeleted Whether or not the bid was deleted after the match was completed.  As of now, this value should
 * always be true, but is represented as a boolean to ensure that future updates that might maintain a bid longer than
 * a single match can be easily supported.
 */
data class ExecuteMatchResponse(
    val askId: String,
    val bidId: String,
    val askDeleted: Boolean,
    val bidDeleted: Boolean,
)
