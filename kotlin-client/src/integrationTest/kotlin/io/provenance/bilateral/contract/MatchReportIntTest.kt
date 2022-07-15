package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.CoinTradeAsk
import io.provenance.bilateral.execute.Bid.CoinTradeBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import org.junit.jupiter.api.Test
import testconfiguration.ContractIntTest
import testconfiguration.functions.newCoins
import java.util.UUID
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class MatchReportIntTest : ContractIntTest() {
    @Test
    fun testSimpleMatchReport() {
        val askId = UUID.randomUUID().toString()
        createAsk(
            createAsk = CreateAsk(
                ask = CoinTradeAsk(
                    id = askId,
                    quote = newCoins(100, "nhash"),
                    base = newCoins(100, "nhash"),
                ),
            ),
        )
        val bidId = UUID.randomUUID().toString()
        createBid(
            createBid = CreateBid(
                bid = CoinTradeBid(
                    id = bidId,
                    quote = newCoins(100, "nhash"),
                    base = newCoins(100, "nhash"),
                ),
            ),
        )
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
    }
}
