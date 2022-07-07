package io.provenance.bilateral.models

import com.fasterxml.jackson.databind.PropertyNamingStrategies.SnakeCaseStrategy
import com.fasterxml.jackson.databind.annotation.JsonNaming
import cosmos.base.v1beta1.CoinOuterClass.Coin

@JsonNaming(SnakeCaseStrategy::class)
data class ContractInfo(
    val admin: String,
    val bindName: String,
    val contractName: String,
    val contractType: String,
    val contractVersion: String,
    val askFee: List<Coin>?,
    val bidFee: List<Coin>?,
)
