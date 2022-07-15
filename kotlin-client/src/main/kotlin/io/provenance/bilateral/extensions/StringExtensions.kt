package io.provenance.bilateral.extensions

import cosmos.base.abci.v1beta1.Abci.TxMsgData
import cosmos.base.abci.v1beta1.Abci.TxResponse
import cosmwasm.wasm.v1beta1.Tx.MsgExecuteContractResponse
import org.bouncycastle.util.encoders.Hex

internal fun TxResponse.executeContractDataToJsonBytes(): ByteArray = Hex
    .decode(this.data)
    .let(TxMsgData::parseFrom)
    .dataList
    .singleOrNull()
    ?.data
    ?.let(MsgExecuteContractResponse::parseFrom)
    ?.data
    ?.toByteArray()
    ?: throw IllegalStateException("Could not parse tx data to contract json. Hex: ${this.data}")
