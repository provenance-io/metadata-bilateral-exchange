package testconfiguration.functions

import io.provenance.bilateral.client.BilateralContractClient
import kotlin.test.DefaultAsserter.fail

fun <T> assertSucceeds(message: String = "Expected function invocation to succeed", fn: () -> T): T =
    try {
        fn()
    } catch(e: Exception) {
        fail(message, e)
    }

fun BilateralContractClient.assertAskExists(askId: String, message: String = "Expected ask [$askId] to exist") {
    assertSucceeds(message) { this.getAsk(askId) }
}

fun BilateralContractClient.assertBidExists(bidId: String, message: String = "Expected bid [$bidId] to exist") {
    assertSucceeds(message) { this.getBid(bidId) }
}

fun BilateralContractClient.assertAskIsDeleted(askId: String, message: String = "Expected ask [$askId] to not be found, but it existed in the contract") {
    try {
        this.getAsk(askId)
        fail(message)
    } catch (_: Exception) {
        // Success - we want an exception to be thrown, indicating the ask is not present
    }
}

fun BilateralContractClient.assertBidIsDeleted(bidId: String, message: String = "Expected bid [$bidId] to not be found, but it existed in the contract") {
    try {
        this.getBid(bidId)
        fail(message)
    } catch (_: Exception) {
        // Success - we want an exception to be thrown, indicating the bid is not present
    }
}
