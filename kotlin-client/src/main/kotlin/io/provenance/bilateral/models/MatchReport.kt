package io.provenance.bilateral.models

import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming

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
