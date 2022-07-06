package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.clearFees
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.setFees
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
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
            receiverAddress = BilateralAccounts.askerAccount.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(3, askFeeDenom),
            receiverAddress = BilateralAccounts.askerAccount.address(),
        )
        bilateralClient.setFees(askFee = newCoins(1, askFeeDenom))
        val quote = newCoins(1000, "somequote")
        val base = newCoins(1000, askerDenom)
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newCoinTrade(
            id = askUuid.toString(),
            base = base,
            quote = quote,
        )
        assertSucceeds("Ask should be created without error") {
            bilateralClient.createAsk(
                createAsk = createAsk,
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), askFeeDenom),
            message = "The correct fee should be removed from the asker's account",
        )
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), askFeeDenom),
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
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(3, bidFeeDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        bilateralClient.setFees(bidFee = newCoins(1, bidFeeDenom))
        val quote = newCoins(1000, bidderDenom)
        val base = newCoins(1000, "somebasedenom")
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newCoinTrade(
            id = bidUuid.toString(),
            base = base,
            quote = quote,
        )
        assertSucceeds("Bid should be created without error") {
            bilateralClient.createBid(
                createBid = createBid,
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidFeeDenom),
            message = "The correct fee should be removed from the bidder's account",
        )
        bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 2,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidFeeDenom),
            message = "The fee should not be refunded after canceling the bid",
        )
        bilateralClient.clearFees()
    }
}
