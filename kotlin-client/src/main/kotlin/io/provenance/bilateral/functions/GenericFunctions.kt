package io.provenance.bilateral.functions

internal fun <T> tryOrNull(fn: () -> T): T? = try {
    fn()
} catch (e: Exception) {
    null
}
