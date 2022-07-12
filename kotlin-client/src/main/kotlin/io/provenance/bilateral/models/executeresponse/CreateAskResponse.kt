package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.AskOrder

data class CreateAskResponse(val askId: String, val askOrder: AskOrder)
