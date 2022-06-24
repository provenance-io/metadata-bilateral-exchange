package testconfiguration.extensions

import com.fasterxml.jackson.core.type.TypeReference
import cosmos.base.abci.v1beta1.Abci.TxResponse
import cosmos.tx.v1beta1.ServiceOuterClass.BroadcastTxResponse
import testconfiguration.models.ProvenanceTxEvents
import testconfiguration.util.ObjectMapperProvider.OBJECT_MAPPER

fun TxResponse.isError(): Boolean = this.code != 0
fun TxResponse.isSuccess(): Boolean = !this.isError()
fun TxResponse.checkIsSuccess(): TxResponse = this.also {
    if (this.isError()) {
        throw IllegalStateException("Broadcast response contained an error! Error log: ${this.rawLog}")
    }
}

/**
 * Expands the rawLow property of the TxResponse into a navigable data structure for logical parsing.
 * When the log corresponds to an error, an exception will be thrown.
 */
fun TxResponse.toProvenanceTxEvents(): List<ProvenanceTxEvents> = OBJECT_MAPPER
    .readValue(this.checkIsSuccess().rawLog, object : TypeReference<List<ProvenanceTxEvents>>() {})

fun BroadcastTxResponse.toProvenanceTxEvents(): List<ProvenanceTxEvents> = txResponse.toProvenanceTxEvents()

fun BroadcastTxResponse.isError(): Boolean = this.txResponse.isError()
fun BroadcastTxResponse.isSuccess(): Boolean = !this.isError()
fun BroadcastTxResponse.checkIsSuccess(): BroadcastTxResponse = this.also { this.txResponse.checkIsSuccess() }

fun BroadcastTxResponse.getCodeIdOrNull(): Long? = toProvenanceTxEvents()
    .flatMap { it.events }
    .singleOrNull { it.type == "store_code" }
    ?.attributes
    ?.singleOrNull { it.key == "code_id" }
    ?.value
    ?.toLongOrNull()

fun BroadcastTxResponse.getCodeId(): Long = getCodeIdOrNull()
    ?: throw IllegalStateException("Unable to retrieve code id from response. Received response log: ${this.txResponse.rawLog}")

fun BroadcastTxResponse.getContractAddressOrNull(): String? = toProvenanceTxEvents()
    .flatMap { it.events }
    .singleOrNull { it.type == "instantiate" }
    ?.attributes
    ?.singleOrNull { it.key == "_contract_address" }
    ?.value

fun BroadcastTxResponse.getContractAddress(): String = getContractAddressOrNull()
    ?: throw IllegalStateException("Unable to retrieve contract address from response.  Received response log: ${this.txResponse.rawLog}")
