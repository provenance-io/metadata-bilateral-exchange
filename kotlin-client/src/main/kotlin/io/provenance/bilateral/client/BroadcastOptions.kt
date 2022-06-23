package io.provenance.bilateral.client

import cosmos.auth.v1beta1.Auth.BaseAccount
import cosmos.base.v1beta1.CoinOuterClass.Coin
import cosmos.tx.v1beta1.ServiceOuterClass.BroadcastMode

data class BroadcastOptions(
    val funds: List<Coin> = emptyList(),
    val broadcastMode: BroadcastMode = BroadcastMode.BROADCAST_MODE_BLOCK,
    val sequenceOffset: Int = 0,
    val gasAdjustment: Double? = null,
    val baseAccount: BaseAccount? = null,
)
