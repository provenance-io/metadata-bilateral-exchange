package io.provenance.bilateral.models.enums

import com.fasterxml.jackson.annotation.JsonProperty

enum class AttributeRequirementType {
    @JsonProperty("all") ALL,
    @JsonProperty("any") ANY,
    @JsonProperty("none") NONE,
}
