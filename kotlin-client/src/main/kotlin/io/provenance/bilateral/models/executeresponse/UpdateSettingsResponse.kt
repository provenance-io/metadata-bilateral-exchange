package io.provenance.bilateral.models.executeresponse

data class UpdateSettingsResponse(
    val newAdminAddress: String?,
    val newAskFee: String?,
    val newBidFee: String?,
)
