package io.provenance.bilateral.models

import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonDeserialize
import com.fasterxml.jackson.databind.annotation.JsonNaming
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import io.provenance.bilateral.models.enums.AttributeRequirementType
import io.provenance.bilateral.serialization.CosmWasmUTCOffsetDateTimeToTimestampSerializer
import io.provenance.bilateral.serialization.CosmWasmUTCTimestampToOffsetDateTimeDeserializer
import java.time.OffsetDateTime

/**
 * Check out CreateAsk.kt for a JSON sample of this output in the request bodies.
 * Note: This requires a custom serializer and deserializer on its timestamp field, "effectiveTime" because the field
 * is in epoch nanos in the smart contract.  As such, it needs to be represented as a String literal numeric value.
 * The custom serializer and deserializer allow lossless conversion between epoch nanos and offset date time.
 */
@JsonNaming(SnakeCaseStrategy::class)
data class RequestDescriptor(
    val description: String? = null,
    @JsonSerialize(using = CosmWasmUTCOffsetDateTimeToTimestampSerializer::class)
    @JsonDeserialize(using = CosmWasmUTCTimestampToOffsetDateTimeDeserializer::class)
    val effectiveTime: OffsetDateTime? = null,
    val attributeRequirement: AttributeRequirement? = null,
)

@JsonNaming(SnakeCaseStrategy::class)
data class AttributeRequirement(
    val attributes: List<String>,
    val requirementType: AttributeRequirementType,
)
