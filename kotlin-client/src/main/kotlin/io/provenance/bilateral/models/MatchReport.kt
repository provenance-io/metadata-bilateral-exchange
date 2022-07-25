package io.provenance.bilateral.models

import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming

/**
 * A payload that denotes whether or not a match can be made for a given [io.provenance.bilateral.models.AskOrder] and
 * [io.provenance.bilateral.models.BidOrder] to allow insight into the process without having to execute the match and
 * examine any successes or exceptions produced by it.
 *
 * @param askId The unique identifier for the ask in the match report.
 * @param bidId The unique identifier for the bid in the match report.
 * @param askExists If true, an [io.provenance.bilateral.models.AskOrder] with [askId] exists.
 * @param bidExists If true, a [io.provenance.bilateral.models.BidOrder] with [bidId] exists.
 * @param standardMatchPossible If true, a match without using the [io.provenance.bilateral.execute.ExecuteMatch.acceptMismatchedBids]
 * flags is possible.
 * @param quoteMismatchMatchPossible If true, a match using the [io.provenance.bilateral.execute.ExecuteMatch.acceptMismatchedBids]
 * is possible.  If [standardMatchPossible] is false, this indicates that a match is only possible using the mismatched
 * bids flag.
 * @param errorMessages If either of [standardMatchPossible] or [quoteMismatchMatchPossible] are false, then this list
 * will be populated with detailed output errors indicating why those matches are not possible.
 */
@JsonNaming(SnakeCaseStrategy::class)
data class MatchReport(
    val askId: String,
    val bidId: String,
    val askExists: Boolean,
    val bidExists: Boolean,
    // Indicates that this is a direct match for ask and bid
    val standardMatchPossible: Boolean,
    // Indicates that this is a direct match on the base, but the quote is different between ask and bid, and not
    // exactly what the asker requested - this could be a higher, lower, or completely different bid quote.  The bidder
    // will still get what the want if this match occurs, so the asker needs to determine if they want what the bidder
    // is offering
    val quoteMismatchMatchPossible: Boolean,
    val errorMessages: List<String>,
)
