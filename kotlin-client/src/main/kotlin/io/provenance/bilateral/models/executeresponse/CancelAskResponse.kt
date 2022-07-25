package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.AskOrder

/**
 * The data response returned after a successful ask cancellation.
 *
 * @param askId The unique identifier of the cancelled ask.
 * @param cancelledAskOrder The [AskOrder] that was held by the contract prior to the ask's cancellation. This is the
 * final remnant of the ask, because on a successful cancellation, the [AskOrder] is fully deleted from the contract.
 */
data class CancelAskResponse(val askId: String, val cancelledAskOrder: AskOrder)
