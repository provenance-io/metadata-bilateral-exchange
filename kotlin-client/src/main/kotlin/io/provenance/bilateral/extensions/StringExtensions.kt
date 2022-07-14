package io.provenance.bilateral.extensions

import cosmos.base.abci.v1beta1.Abci
import cosmos.base.abci.v1beta1.Abci.TxResponse
import cosmwasm.wasm.v1beta1.Tx.MsgExecuteContractResponse
import org.bouncycastle.util.encoders.Hex

fun TxResponse.responseDataToJsonBytes(): ByteArray = Hex
    .decode(this.data)
    .let { Abci.TxMsgData.parseFrom(it) }
    .dataList
    .singleOrNull()
    ?.data
    ?.let { MsgExecuteContractResponse.parseFrom(it) }
    ?.data
    ?.toByteArray()
    ?: throw IllegalStateException("Response bytes returned could not be parsed into a single MsgData with the proper formatting")
