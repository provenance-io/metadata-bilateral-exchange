package io.provenance.bilateral.extensions

import cosmos.tx.v1beta1.ServiceOuterClass.BroadcastTxResponse
import io.provenance.bilateral.exceptions.ProvenanceEventParsingException
import tendermint.abci.Types.Event

/**
 * Fetches a single event of type wasm from the tx response events list.  This is expected in every successful response
 * created by executing a smart contract.
 */
internal fun BroadcastTxResponse.singleWasmEvent(): Event = txResponse
    .eventsList
    .singleOrNull { it.type == "wasm" }
    ?: throw ProvenanceEventParsingException("Expected a single wasm event to be emitted by the Metadata Bilateral Exchange smart contract. Got log: ${txResponse.rawLog}")

/**
 * Attempts to fetch an attribute from an event by its key.  If the event value is not available, null is returned.
 */
internal fun Event.attributeOrNull(name: String): String? = attributesList.singleOrNull { it.key.toStringUTF8() == name }?.value?.toStringUTF8()

/**
 * Attempts to fetch an attribute from an event by its key.  If the event value is not available, an exception is thrown.
 */
internal fun Event.attribute(name: String): String = attributeOrNull(name)
    ?: throw ProvenanceEventParsingException("Failed to find attribute by name [$name] in list: ${attributesList.map { it.key.toStringUTF8() }}")
