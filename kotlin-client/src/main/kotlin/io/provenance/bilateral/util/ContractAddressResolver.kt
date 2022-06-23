package io.provenance.bilateral.util

import io.provenance.client.grpc.PbClient
import io.provenance.client.protobuf.extensions.resolveAddressForName

sealed interface ContractAddressResolver {
    class ProvidedAddress(val address: String) : ContractAddressResolver
    class FromName(val name: String) : ContractAddressResolver

    fun getAddress(pbClient: PbClient): String = when (this) {
        is ProvidedAddress -> this.address
        is FromName -> pbClient.nameClient.resolveAddressForName(this.name)
    }
}
