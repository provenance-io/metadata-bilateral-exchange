package io.provenance.bilateral.extensions

import com.google.protobuf.ByteString

internal fun ByteString.toRawStringContent(): String = this.toString(Charsets.UTF_8)
