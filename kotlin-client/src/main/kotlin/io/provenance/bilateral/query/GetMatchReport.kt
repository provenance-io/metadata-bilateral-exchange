package io.provenance.bilateral.query

import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.interfaces.ContractQueryMsg

@JsonNaming(SnakeCaseStrategy::class)
data class GetMatchReport(val getMatchReport: Body) : ContractQueryMsg {
    @JsonNaming(SnakeCaseStrategy::class)
    data class Body(val askId: String, val bidId: String)

    companion object {
        fun new(askId: String, bidId: String): GetMatchReport = GetMatchReport(
            getMatchReport = Body(askId, bidId)
        )
    }
}
