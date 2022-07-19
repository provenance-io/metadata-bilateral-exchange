package io.provenance.bilateral.query

import com.fasterxml.jackson.annotation.JsonIgnore
import com.fasterxml.jackson.annotation.JsonRootName
import com.fasterxml.jackson.annotation.JsonSubTypes
import com.fasterxml.jackson.annotation.JsonTypeId
import com.fasterxml.jackson.annotation.JsonTypeInfo
import com.fasterxml.jackson.annotation.JsonTypeName
import com.fasterxml.jackson.annotation.JsonValue
import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonAppend
import com.fasterxml.jackson.databind.annotation.JsonNaming
import com.fasterxml.jackson.databind.annotation.JsonSerialize
import io.provenance.bilateral.interfaces.BilateralContractQueryMsg
import io.provenance.bilateral.models.enums.BilateralRequestType
import io.provenance.bilateral.serialization.CosmWasmBigIntegerToUintSerializer
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
    }.let { searchTypeString -> "searchType = $searchTypeString, pageSize = [${pageSize ?: "DEFAULT"}], pageNumber = [${pageNumber ?: "DEFAULT"}]" }
}

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("search_asks")
data class SearchAsks(val search: ContractSearchRequest) : BilateralContractQueryMsg {
    override fun toLoggingString(): String = "searchAsks, ${search.getLoggingSuffix()}"
}

@JsonNaming(SnakeCaseStrategy::class)
@JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
@JsonTypeName("search_bids")
data class SearchBids(val search: ContractSearchRequest) : BilateralContractQueryMsg {
    override fun toLoggingString(): String = "searchBids, ${search.getLoggingSuffix()}"
}

sealed interface ContractSearchType {
    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeName("all")
    object All : ContractSearchType {
        @JsonValue
        fun serializeAs(): String = "all"
    }

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
    @JsonTypeName("value_type")
    class Type private constructor(val valueType: Body) : ContractSearchType {
        constructor(valueType: BilateralRequestType) : this(Body(valueType))

        class Body(val valueType: BilateralRequestType)
    }

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
    @JsonTypeName("id")
    class Id private constructor(val id: Body) : ContractSearchType {
        constructor(id: String) : this(Body(id))

        class Body(val id: String)
    }

    @JsonNaming(SnakeCaseStrategy::class)
    @JsonTypeInfo(include = JsonTypeInfo.As.WRAPPER_OBJECT, use = JsonTypeInfo.Id.NAME)
    @JsonTypeName("owner")
    class Owner private constructor(val owner: Body) : ContractSearchType {
        constructor(owner: String) : this(Body(owner))

        class Body(val owner: String)
    }
}
