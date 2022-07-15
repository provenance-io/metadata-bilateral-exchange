package io.provenance.bilateral.query

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.annotation.JsonValue
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonDeserialize
import com.fasterxml.jackson.databind.annotation.JsonNaming
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import io.provenance.bilateral.interfaces.ContractQueryMsg
import io.provenance.bilateral.serialization.CosmWasmBigIntegerToUintSerializer
import io.provenance.bilateral.serialization.CosmWasmUintToBigIntegerDeserializer
import java.math.BigInteger

@JsonNaming(SnakeCaseStrategy::class)
data class ContractSearchRequest(
    val searchType: ContractSearchType,
    @JsonSerialize(using = CosmWasmBigIntegerToUintSerializer::class)
    val pageSize: BigInteger? = null,
    @JsonSerialize(using = CosmWasmBigIntegerToUintSerializer::class)
    val pageNumber: BigInteger? = null,
) {
    companion object {
        val DEFAULT_PAGE_SIZE: BigInteger = BigInteger.TEN
        val MAX_PAGE_SIZE: BigInteger = 25.toBigInteger()
        val MIN_PAGE_SIZE: BigInteger = BigInteger.ONE
        val DEFAULT_PAGE_NUMBER: BigInteger = BigInteger.ONE
        val MIN_PAGE_NUMBER: BigInteger = BigInteger.ONE
    }

    internal fun searchAsks(): SearchAsks = SearchAsks(this)
    internal fun searchBids(): SearchBids = SearchBids(this)

    @JsonIgnore
    internal fun getLoggingSuffix(): String = when (this.searchType) {
        is ContractSearchType.All -> "[all]"
        is ContractSearchType.Type -> "[type], type = [${this.searchType.valueType}]"
        is ContractSearchType.Id -> "[id], id = [${this.searchType.id}]"
        is ContractSearchType.Owner -> "[owner], owner = [${this.searchType.owner}]"
    }.let { searchTypeString -> "searchType = $searchTypeString, pageSize = [${pageSize ?: "DEFAULT"}, pageNumber = [${pageNumber ?: "DEFAULT"}]" }
}

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("search_asks")
data class SearchAsks(val search: ContractSearchRequest) : ContractQueryMsg {
    override fun toLoggingString(): String = "searchAsks, ${search.getLoggingSuffix()}"
}

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("search_bids")
data class SearchBids(val search: ContractSearchRequest) : ContractQueryMsg {
    override fun toLoggingString(): String = "searchBids, ${search.getLoggingSuffix()}"
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
    @JsonDeserialize(using = CosmWasmUintToBigIntegerDeserializer::class)
    val pageNumber: BigInteger,
    @JsonDeserialize(using = CosmWasmUintToBigIntegerDeserializer::class)
    val pageSize: BigInteger,
    @JsonDeserialize(using = CosmWasmUintToBigIntegerDeserializer::class)
    val totalPages: BigInteger,
)

enum class BilateralRequestType(val contractName: String) {
    COIN_TRADE("coin_trade"),
    MARKER_TRADE("marker_trade"),
    MARKER_SHARE_SALE("marker_share_sale"),
    SCOPE_TRADE("scope_trade"),
}
