package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.AskOrder

/**
 * The data response returned after an ask is successfully created.
 *
 * @param askId The unique identifier of the created ask.
 * @param askOrder The [AskOrder] created by the request.  This value directly represents the current state of the order
 * in the contract.
 */
data class CreateAskResponse(val askId: String, val askOrder: AskOrder)
