package io.provenance.bilateral.models

import com.fasterxml.jackson.databind.PropertyNamingStrategies
import com.fasterxml.jackson.databind.annotation.JsonDeserialize
import com.fasterxml.jackson.databind.annotation.JsonNaming
import io.provenance.bilateral.serialization.CosmWasmUintToBigIntegerDeserializer
import java.math.BigInteger

@JsonNaming(PropertyNamingStrategies.SnakeCaseStrategy::class)
data class ContractSearchResult<T>(
    val results: List<T>,
    @JsonDeserialize(using = CosmWasmUintToBigIntegerDeserializer::class)
    val pageNumber: BigInteger,
    @JsonDeserialize(using = CosmWasmUintToBigIntegerDeserializer::class)
    val pageSize: BigInteger,
    @JsonDeserialize(using = CosmWasmUintToBigIntegerDeserializer::class)
    val totalPages: BigInteger,
)
