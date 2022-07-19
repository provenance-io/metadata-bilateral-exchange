package io.provenance.bilateral.models.executeresponse

/**
 * The data response returned after the contract settings are successfully updated.
 *
 * @param newAdminAddress If the admin address was changed, this value will be non-null and contain the new admin's
 * bech32 address.
 * @param newAskFee Represents a formatted display of the new ask fee amount.  If no fee is set, the returned value will
 * be "none."
 * @param newBidFee Represents a formatted display of the new bid fee amount.  If no fee is set, the returned value will
 * be "none."
 */
data class UpdateSettingsResponse(
    val newAdminAddress: String?,
    val newAskFee: String?,
    val newBidFee: String?,
)
