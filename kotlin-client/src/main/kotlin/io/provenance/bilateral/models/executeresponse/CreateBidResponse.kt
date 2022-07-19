package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.BidOrder

/**
 * The data response returned after a bid is successfully created.
 *
 * @param bidId The unique identifier of the created bid.
 * @param bidOrder The [BidOrder] created by the request.  This value directly represents the current state of the order
 * in the contract.
 */
data class CreateBidResponse(val bidId: String, val bidOrder: BidOrder)
