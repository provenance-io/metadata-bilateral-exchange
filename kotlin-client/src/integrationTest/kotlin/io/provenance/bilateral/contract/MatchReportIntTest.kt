package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.util.UUID
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class MatchReportIntTest : ContractIntTest() {
    @Test
    fun testSimpleMatchReport() {
        val askId = UUID.randomUUID().toString()
        bilateralClient.createAsk(
            createAsk = CreateAsk.newCoinTrade(
                id = askId,
                quote = newCoins(100, "nhash"),
                base = newCoins(100, "nhash"),
            ),
            signer = BilateralAccounts.askerAccount,
        )
        bilateralClient.assertAskExists(askId)
        val bidId = UUID.randomUUID().toString()
        bilateralClient.createBid(
            createBid = CreateBid.newCoinTrade(
                id = bidId,
                quote = newCoins(100, "nhash"),
                base = newCoins(100, "nhash"),
            ),
            signer = BilateralAccounts.bidderAccount,
        )
        bilateralClient.assertBidExists(bidId)
        val matchReport = bilateralClient.getMatchReport(askId, bidId)
        assertEquals(
            expected = askId,
            actual = matchReport.askId,
            message = "The ask ids should match",
        )
        assertEquals(
            expected = bidId,
            actual = matchReport.bidId,
            message = "The bid ids should match",
        )
        assertTrue(
            actual = matchReport.askExists,
            message = "The ask should be marked as existing",
        )
        assertTrue(
            actual = matchReport.bidExists,
            message = "The bid should be marked as existing",
        )
        assertTrue(
            actual = matchReport.standardMatchPossible,
            message = "The report should indicate that a standard match is possible",
        )
        assertTrue(
            actual = matchReport.quoteMismatchMatchPossible,
            message = "The report should indicate that a quote mismatch-enabled match is possible",
        )
        assertTrue(
            actual = matchReport.errorMessages.isEmpty(),
            message = "The report should not include any error messages, but found: ${matchReport.errorMessages}",
        )
        // Cleanup outstanding ask and bid
        bilateralClient.cancelAsk(askId, BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askId)
        bilateralClient.cancelBid(bidId, BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidId)
    }
}
