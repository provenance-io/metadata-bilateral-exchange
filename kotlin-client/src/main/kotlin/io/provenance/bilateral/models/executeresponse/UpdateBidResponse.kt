package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.BidOrder

data class UpdateBidResponse(val bidId: String, val updatedBidOrder: BidOrder)
