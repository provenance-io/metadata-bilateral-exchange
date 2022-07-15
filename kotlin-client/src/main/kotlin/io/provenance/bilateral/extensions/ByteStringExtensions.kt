package io.provenance.bilateral.extensions

import com.google.protobuf.ByteString

internal fun ByteString.toStringUTF8(): String = this.toString(Charsets.UTF_8)
