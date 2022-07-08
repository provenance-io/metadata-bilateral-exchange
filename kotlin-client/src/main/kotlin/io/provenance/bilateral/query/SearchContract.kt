package io.provenance.bilateral.query

import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.annotation.JsonValue
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import io.provenance.bilateral.interfaces.ContractQueryMsg
import io.provenance.bilateral.serialization.CosmWasmBigIntegerToUintSerializer
import java.math.BigInteger

/**
 * See ContractSearchType for JSON payloads for each different type of ask search.
 */
@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("search_asks")
data class SearchAsks(val search: ContractSearchRequest) : ContractQueryMsg

/**
 * See ContractSearchType for JSON payloads for each different type of bid search.
 */
@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("search_bids")
data class SearchBids(val search: ContractSearchRequest) : ContractQueryMsg

@JsonNaming(SnakeCaseStrategy::class)
data class ContractSearchRequest(
    val searchType: ContractSearchType,
    @JsonSerialize(using = CosmWasmBigIntegerToUintSerializer::class)
    val pageSize: BigInteger? = null,
    @JsonSerialize(using = CosmWasmBigIntegerToUintSerializer::class)
    val pageNumber: BigInteger? = null,
) {
    internal fun searchAsks(): SearchAsks = SearchAsks(this)
    internal fun searchBids(): SearchBids = SearchBids(this)
}

sealed interface ContractSearchType {
    @JsonNaming(SnakeCaseStrategy::class)
    object All : ContractSearchType {
        @JsonValue
        fun serializeAs(): String = "all"
    }

    @JsonNaming(SnakeCaseStrategy::class)
    data class Type(val valueType: Body) : ContractSearchType {
        @JsonNaming(SnakeCaseStrategy::class)
        data class Body(val valueType: String)
    }

    @JsonNaming(SnakeCaseStrategy::class)
    data class Id(val id: Body) : ContractSearchType {
        @JsonNaming(SnakeCaseStrategy::class)
        data class Body(val id: String)
    }

    @JsonNaming(SnakeCaseStrategy::class)
    data class Owner(val owner: Body) : ContractSearchType {
        @JsonNaming(SnakeCaseStrategy::class)
        data class Body(val owner: String)
    }

    companion object {
        fun all(): ContractSearchType = All
        fun byType(type: BilateralRequestType): ContractSearchType = Type(Type.Body(type.contractName))
        fun byId(id: String): ContractSearchType = Id(Id.Body(id))
        fun byOwner(owner: String): ContractSearchType = Owner(Owner.Body(owner))
    }
}

@JsonNaming(SnakeCaseStrategy::class)
data class ContractSearchResult<T>(
    val results: List<T>,
    val pageNumber: Int,
    val pageSize: Int,
    val totalPages: Int,
)

enum class BilateralRequestType(val contractName: String) {
    COIN_TRADE("coin_trade"),
    MARKER_TRADE("marker_trade"),
    MARKER_SHARE_SALE("marker_share_sale"),
    SCOPE_TRADE("scope_trade"),
}
