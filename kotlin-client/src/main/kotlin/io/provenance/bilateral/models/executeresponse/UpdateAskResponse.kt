package io.provenance.bilateral.models.executeresponse

import io.provenance.bilateral.models.AskOrder

data class UpdateAskResponse(val askId: String, val updatedAskOrder: AskOrder)
