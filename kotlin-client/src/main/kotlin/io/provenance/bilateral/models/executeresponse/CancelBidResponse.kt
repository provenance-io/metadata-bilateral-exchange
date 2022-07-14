package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.BidOrder

data class CancelBidResponse(val bidId: String, val cancelledBidOrder: BidOrder)
