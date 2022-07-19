package io.provenance.bilateral.models.enums

import com.fasterxml.jackson.annotation.JsonProperty

enum class BilateralRequestType {
    @JsonProperty("coin_trade") COIN_TRADE,
    @JsonProperty("marker_trade") MARKER_TRADE,
    @JsonProperty("marker_share_sale") MARKER_SHARE_SALE,
    @JsonProperty("scope_trade") SCOPE_TRADE,
}
