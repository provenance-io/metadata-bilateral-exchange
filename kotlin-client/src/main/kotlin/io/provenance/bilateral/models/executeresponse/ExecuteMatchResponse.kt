package io.provenance.bilateral.models.executeresponse

data class ExecuteMatchResponse(
    val askId: String,
    val bidId: String,
    val askDeleted: Boolean,
    val bidDeleted: Boolean,
)
