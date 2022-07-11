package io.provenance.bilateral.interfaces

import com.fasterxml.jackson.databind.ObjectMapper
import com.google.protobuf.ByteString
import io.provenance.scope.util.toByteString

interface ContractMsg {
    fun toLoggingString(): String

    fun toJsonByteString(objectMapper: ObjectMapper): ByteString = objectMapper.writeValueAsString(this).toByteString()
}
