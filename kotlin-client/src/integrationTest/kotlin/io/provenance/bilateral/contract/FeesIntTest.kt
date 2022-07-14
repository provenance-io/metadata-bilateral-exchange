package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.CoinTradeAsk
import io.provenance.bilateral.execute.Bid.CoinTradeBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import org.junit.jupiter.api.Test
import testconfiguration.extensions.clearFees
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.setFees
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
import testconfiguration.ContractIntTest
import java.util.UUID
import kotlin.test.assertEquals

class FeesIntTest : ContractIntTest() {
    @Test
    fun testCreateAskWithFee() {
        val askerDenom = "cointradecreateaskwithfeebase"
        val askFeeDenom = "cointradecreateaskwithfeefee"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, askerDenom),
            receiverAddress = asker.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(3, askFeeDenom),
            receiverAddress = asker.address(),
        )
        bilateralClient.setFees(askFee = newCoins(1, askFeeDenom))
        val quote = newCoins(1000, "somequote")
        val base = newCoins(1000, askerDenom)
        val askUuid = UUID.randomUUID()
        assertSucceeds("Ask should be created without error") {
            createAsk(
                createAsk = CreateAsk(
                    ask = CoinTradeAsk(
                        id = askUuid.toString(),
                        base = base,
                        quote = quote,
                    ),
                ),
            )
        }
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(asker.address(), askFeeDenom),
            message = "The correct fee should be removed from the asker's account",
        )
        cancelAsk(askUuid.toString())
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(asker.address(), askFeeDenom),
            message = "The fee should not be refunded after canceling the ask",
        )
        bilateralClient.clearFees()
    }

    @Test
    fun testCreateBidWithFee() {
        val bidderDenom = "cointradecreatebidwithfeebase"
        val bidFeeDenom = "cointradecreatebidwithfeefee"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, bidderDenom),
            receiverAddress = bidder.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(3, bidFeeDenom),
            receiverAddress = bidder.address(),
        )
        bilateralClient.setFees(bidFee = newCoins(1, bidFeeDenom))
        val quote = newCoins(1000, bidderDenom)
        val base = newCoins(1000, "somebasedenom")
        val bidUuid = UUID.randomUUID()
        assertSucceeds("Bid should be created without error") {
            createBid(
                createBid = CreateBid(
                    bid = CoinTradeBid(
                        id = bidUuid.toString(),
                        base = base,
                        quote = quote,
                    ),
                ),
            )
        }
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(bidder.address(), bidFeeDenom),
            message = "The correct fee should be removed from the bidder's account",
        )
        cancelBid(bidUuid.toString())
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(bidder.address(), bidFeeDenom),
            message = "The fee should not be refunded after canceling the bid",
        )
        bilateralClient.clearFees()
    }
}
