package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.CoinTradeAsk
import io.provenance.bilateral.execute.Bid.CoinTradeBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.execute.UpdateAsk
import io.provenance.bilateral.execute.UpdateBid
import io.provenance.bilateral.models.AttributeRequirement
import io.provenance.bilateral.models.AttributeRequirementType
import io.provenance.bilateral.models.RequestDescriptor
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.getBalanceMap
import testconfiguration.extensions.testGetCoinTrade
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.time.OffsetDateTime
import java.util.UUID
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CoinTradeIntTest : ContractIntTest() {
    @Test
    fun testCoinTradeCompleteFlow() {
        val askerDenom = "cointradecompleteflowa"
        val bidderDenom = "cointradecompleteflowb"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, askerDenom),
            receiverAddress = BilateralAccounts.askerAccount.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val quote = newCoins(1000, bidderDenom)
        val base = newCoins(1000, askerDenom)
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk(
            ask = CoinTradeAsk(
                id = askUuid.toString(),
                base = base,
                quote = quote,
            ),
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(listOf("a.pb", "b.pb"), AttributeRequirementType.NONE),
            )
        )
        val createAskResponse = assertSucceeds("Ask should be created without error") {
            bilateralClient.createAsk(
                createAsk = createAsk,
                signer = BilateralAccounts.askerAccount,
            )
        }
        assertEquals(
            expected = askUuid.toString(),
            actual = createAskResponse.askId,
            message = "Expected the ask response to include the correct ask id",
        )
        assertEquals(
            expected = bilateralClient.getAsk(createAskResponse.askId),
            actual = createAskResponse.askOrder,
            message = "Expected the returned ask order to be the value stored in the contract",
        )
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = 0,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), askerDenom),
            message = "The asker account's entire coin balance should be held in escrow after creating an ask",
        )
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = CoinTradeBid(
                id = bidUuid.toString(),
                base = base,
                quote = quote,
            ),
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(listOf("c.pb"), AttributeRequirementType.NONE),
            ),
        )
        val createBidResponse = assertSucceeds("Bid should be created without error") {
            bilateralClient.createBid(
                createBid = createBid,
                signer = BilateralAccounts.bidderAccount,
            )
        }
        assertEquals(
            expected = bidUuid.toString(),
            actual = createBidResponse.bidId,
            message = "Expected the response for the bid order to include the correct id",
        )
        assertEquals(
            expected = bilateralClient.getBid(createBidResponse.bidId),
            actual = createBidResponse.bidOrder,
            message = "Expected the returned bid order to be the value stored in the contract",
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The bidder account's entire coin balance should be held in escrow after creating a bid",
        )
        val executeMatch = ExecuteMatch(
            askId = askUuid.toString(),
            bidId = bidUuid.toString(),
        )
        val executeMatchResponse = assertSucceeds("Match should be executed without error") {
            bilateralClient.executeMatch(
                executeMatch = executeMatch,
                signer = BilateralAccounts.adminAccount,
            )
        }
        assertEquals(
            expected = askUuid.toString(),
            actual = executeMatchResponse.askId,
            message = "Expected the ask id in the match response to be correct",
        )
        assertEquals(
            expected = bidUuid.toString(),
            actual = executeMatchResponse.bidId,
            message = "Expected the bid id in the match response to be correct",
        )
        assertTrue(
            actual = executeMatchResponse.askDeleted,
            message = "Expected the response to indicate that the ask was deleted",
        )
        assertTrue(
            actual = executeMatchResponse.bidDeleted,
            message = "Expected the response to indicate that the bid was deleted",
        )
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        val askerBalances = pbClient.getBalanceMap(BilateralAccounts.askerAccount.address())
        assertNull(
            actual = askerBalances[askerDenom],
            message = "The asker should no longer have any [$askerDenom] because it should have been sent to the bidder",
        )
        assertEquals(
            expected = 1000L,
            actual = askerBalances[bidderDenom],
            message = "The asker should have received all of the bidder's [$bidderDenom]",
        )
        val bidderBalances = pbClient.getBalanceMap(BilateralAccounts.bidderAccount.address())
        assertNull(
            actual = bidderBalances[bidderDenom],
            message = "The bidder should no longer have any [$bidderDenom] because it should have been sent to the asker",
        )
        assertEquals(
            expected = 1000L,
            actual = bidderBalances[askerDenom],
            message = "The bidder should have received all of the asker's [$askerDenom]"
        )
    }

    @Test
    fun testMismatchedBidExecute() {
        val askerDenom = "cointrademismatchedask"
        val bidderDenom = "cointrademismatchedbid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, askerDenom),
            receiverAddress = BilateralAccounts.askerAccount.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val askQuote = newCoins(1000, "someotherthing")
        val bidQuote = newCoins(1000, bidderDenom)
        val base = newCoins(1000, askerDenom)
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk(
            ask = CoinTradeAsk(
                id = askUuid.toString(),
                base = base,
                quote = askQuote,
            ),
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(listOf("a.pb", "b.pb"), AttributeRequirementType.NONE),
            ),
        )
        assertSucceeds("Ask should be created without error") {
            bilateralClient.createAsk(
                createAsk = createAsk,
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = 0,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), askerDenom),
            message = "The asker account's entire coin balance should be held in escrow after creating an ask",
        )
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = CoinTradeBid(
                id = bidUuid.toString(),
                base = base,
                quote = bidQuote,
            ),
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(listOf("c.pb"), AttributeRequirementType.NONE),
            ),
        )
        assertSucceeds("Bid should be created without error") {
            bilateralClient.createBid(
                createBid = createBid,
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The bidder account's entire coin balance should be held in escrow after creating a bid",
        )
        val executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString())
        assertFails("Match should fail because the quotes don't match") {
            bilateralClient.executeMatch(
                executeMatch = executeMatch,
                signer = BilateralAccounts.adminAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        bilateralClient.assertBidExists(bidUuid.toString())
        assertSucceeds("Match should succeed because it was manually allowed") {
            bilateralClient.executeMatch(
                executeMatch = executeMatch.copy(acceptMismatchedBids = true),
                signer = BilateralAccounts.adminAccount,
            )
        }
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        val askerBalances = pbClient.getBalanceMap(BilateralAccounts.askerAccount.address())
        assertNull(
            actual = askerBalances[askerDenom],
            message = "The asker should no longer have any [$askerDenom] because it should have been sent to the bidder",
        )
        assertEquals(
            expected = 1000L,
            actual = askerBalances[bidderDenom],
            message = "The asker should have received all of the bidder's [$bidderDenom]",
        )
        val bidderBalances = pbClient.getBalanceMap(BilateralAccounts.bidderAccount.address())
        assertNull(
            actual = bidderBalances[bidderDenom],
            message = "The bidder should no longer have any [$bidderDenom] because it should have been sent to the asker",
        )
        assertEquals(
            expected = 1000L,
            actual = bidderBalances[askerDenom],
            message = "The bidder should have received all of the asker's [$askerDenom]"
        )
    }

    @Test
    fun testCancelAsk() {
        val askerDenom = "cointradecancelask"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(amount = 100, denom = askerDenom),
            receiverAddress = BilateralAccounts.askerAccount.address(),
        )
        val askUuid = UUID.randomUUID()
        bilateralClient.createAsk(
            createAsk = CreateAsk(
                ask = CoinTradeAsk(
                    id = askUuid.toString(),
                    quote = newCoins(150, "nhash"),
                    base = newCoins(100, askerDenom),
                ),
            ),
            signer = BilateralAccounts.askerAccount,
        )
        val askOrder = bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), askerDenom),
            message = "The base should be withdrawn from the asker's account",
        )
        val response = bilateralClient.cancelAsk(
            askId = askUuid.toString(),
            signer = BilateralAccounts.askerAccount,
        )
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "The correct ask id should be returned in the response",
        )
        assertEquals(
            expected = askOrder,
            actual = response.cancelledAskOrder,
            message = "The cancelled ask order should be included in the response",
        )
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), askerDenom),
            message = "After cancelling an ask, the base should be returned to the asker",
        )
    }

    @Test
    fun testCancelBid() {
        val bidderDenom = "cointradecancelbid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(amount = 100, denom = bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val bidUuid = UUID.randomUUID()
        bilateralClient.createBid(
            createBid = CreateBid(
                bid = CoinTradeBid(
                    id = bidUuid.toString(),
                    quote = newCoins(100, bidderDenom),
                    base = newCoins(150, "nhash"),
                ),
            ),
            signer = BilateralAccounts.bidderAccount
        )
        val bidOrder = bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The quote should be withdrawn from the bidder's account",
        )
        val response = bilateralClient.cancelBid(
            bidId = bidUuid.toString(),
            signer = BilateralAccounts.bidderAccount,
        )
        assertEquals(
            expected = bidUuid.toString(),
            actual = response.bidId,
            message = "The correct bid id should be returned in the response",
        )
        assertEquals(
            expected = bidOrder,
            actual = response.cancelledBidOrder,
            message = "The cancelled bid order should be included in the response",
        )
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "After canceling a bid, the quote should be returned to the bidder",
        )
    }

    @Test
    fun testUpdateAsk() {
        val baseDenom = "updatecointradeaskbase"
        val base2Denom = "updatecointradeaskbase2"
        val quoteDenom = "updatecointradeaskquote"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, baseDenom),
            receiverAddress = BilateralAccounts.askerAccount.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, base2Denom),
            receiverAddress = BilateralAccounts.askerAccount.address(),
        )
        val askUuid = UUID.randomUUID()
        assertSucceeds("Initial ask should be created without error") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = CoinTradeAsk(
                        id = askUuid.toString(),
                        base = newCoins(100, baseDenom),
                        quote = newCoins(100, quoteDenom),
                    ),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = 900,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), baseDenom),
            message = "The asker's account should have been debited of its base denom",
        )
        val response = assertSucceeds("Update ask should succeed without error") {
            bilateralClient.updateAsk(
                updateAsk = UpdateAsk(
                    ask = CoinTradeAsk(
                        id = askUuid.toString(),
                        base = newCoins(600, base2Denom),
                        quote = newCoins(200, quoteDenom),
                    )
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "The update response should include the correct ask id",
        )
        assertEquals(
            expected = bilateralClient.getAsk(response.askId),
            actual = response.updatedAskOrder,
            message = "The update response should include the updated ask order",
        )
        val updatedCollateral = response.updatedAskOrder.testGetCoinTrade()
        assertEquals(
            expected = newCoins(600, base2Denom),
            actual = updatedCollateral.base,
            message = "Expected the base to be properly updated",
        )
        assertEquals(
            expected = newCoins(200, quoteDenom),
            actual = updatedCollateral.quote,
            message = "Expected the quote to be properly updated",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), baseDenom),
            message = "The asker's base denom should have been refunded from the initial ask, restoring their balance to max",
        )
        assertEquals(
            expected = 400,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), base2Denom),
            message = "The asker's account should have been debited of its base2 denom",
        )
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
    }

    @Test
    fun testUpdateBid() {
        val quoteDenom = "updatecointradeaskquote"
        val quote2Denom = "updatecointradeaskquote2"
        val baseDenom = "updatecointradeaskbase"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, quoteDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, quote2Denom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val bidUuid = UUID.randomUUID()
        assertSucceeds("Initial ask should be created without error") {
            bilateralClient.createBid(
                createBid = CreateBid(
                    bid = CoinTradeBid(
                        id = bidUuid.toString(),
                        quote = newCoins(100, quoteDenom),
                        base = newCoins(100, baseDenom),
                    ),
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 900,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The bidder's account should have been debited of its quote denom",
        )
        val response = assertSucceeds("Update bid should succeed without error") {
            bilateralClient.updateBid(
                updateBid = UpdateBid(
                    bid = CoinTradeBid(
                        id = bidUuid.toString(),
                        quote = newCoins(600, quote2Denom),
                        base = newCoins(200, baseDenom),
                    )
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = bidUuid.toString(),
            actual = response.bidId,
            message = "The update response should include the correct bid id",
        )
        assertEquals(
            expected = bilateralClient.getBid(response.bidId),
            actual = response.updatedBidOrder,
            message = "The update response should include the correct bid order",
        )
        val updatedCollateral = response.updatedBidOrder.testGetCoinTrade()
        assertEquals(
            expected = newCoins(600, quote2Denom),
            actual = updatedCollateral.quote,
            message = "Expected the quote to be properly updated",
        )
        assertEquals(
            expected = newCoins(200, baseDenom),
            actual = updatedCollateral.base,
            message = "Expected the base to be properly updated",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The bidder's quote denom should have been refunded from the initial bid, restoring their balance to max",
        )
        assertEquals(
            expected = 400,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quote2Denom),
            message = "The bidder's account should have been debited of its quote2 denom",
        )
        bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }
}
