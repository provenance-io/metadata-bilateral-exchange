package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.AskOrder

data class CancelAskResponse(val askId: String, val cancelledAskOrder: AskOrder)
