package io.provenance.bilateral.extensions

import cosmos.tx.v1beta1.ServiceOuterClass.BroadcastTxResponse
import io.provenance.bilateral.exceptions.ProvenanceEventParsingException
import tendermint.abci.Types.Event

fun BroadcastTxResponse.singleWasmEvent(): Event = txResponse
    .eventsList
    .singleOrNull { it.type == "wasm" }
    ?: throw ProvenanceEventParsingException("Expected a single wasm event to be emitted by the Metadata Bilateral Exchange smart contract. Got log: ${txResponse.rawLog}")

fun Event.attributeOrNull(name: String): String? = attributesList.singleOrNull { it.key.toRawStringContent() == name }?.value?.toRawStringContent()

fun Event.attribute(name: String): String = attributeOrNull(name)
    ?: throw ProvenanceEventParsingException("Failed to find attribute by name [$name] in list: ${attributesList.map { it.key.toRawStringContent() }}")
