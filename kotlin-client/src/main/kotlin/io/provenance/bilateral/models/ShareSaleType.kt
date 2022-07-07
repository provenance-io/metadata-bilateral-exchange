package io.provenance.bilateral.models

import com.fasterxml.jackson.annotation.JsonProperty

/**
 * See CreateAsk for a JSON payload that includes this object's use.
 */
enum class ShareSaleType {
    @JsonProperty("single_transaction")
    SINGLE_TRANSACTION,
    @JsonProperty("multiple_transactions")
    MULTIPLE_TRANSACTIONS,
}
