package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.BidOrder

data class CreateBidResponse(val bidId: String, val bidOrder: BidOrder)
