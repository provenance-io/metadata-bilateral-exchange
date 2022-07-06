package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.AttributeRequirement
import io.provenance.bilateral.models.AttributeRequirementType
import io.provenance.bilateral.models.RequestDescriptor
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.getBalanceMap
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
        val createAsk = CreateAsk.newCoinTrade(
            id = askUuid.toString(),
            base = base,
            quote = quote,
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(listOf("a.pb", "b.pb"), AttributeRequirementType.NONE),
            )
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
        val createBid = CreateBid.newCoinTrade(
            id = bidUuid.toString(),
            base = base,
            quote = quote,
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
        val executeMatch = ExecuteMatch(
            askId = askUuid.toString(),
            bidId = bidUuid.toString(),
        )
        assertSucceeds("Match should be executed without error") {
            bilateralClient.executeMatch(
                executeMatch = executeMatch,
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
        val createAsk = CreateAsk.newCoinTrade(
            id = askUuid.toString(),
            base = base,
            quote = askQuote,
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
        val createBid = CreateBid.newCoinTrade(
            id = bidUuid.toString(),
            base = base,
            quote = bidQuote,
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
            createAsk = CreateAsk.newCoinTrade(
                id = askUuid.toString(),
                quote = newCoins(150, "nhash"),
                base = newCoins(100, askerDenom),
            ),
            signer = BilateralAccounts.askerAccount,
        )
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), askerDenom),
            message = "The base should be withdrawn from the asker's account",
        )
        bilateralClient.cancelAsk(
            askId = askUuid.toString(),
            signer = BilateralAccounts.askerAccount,
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
            createBid = CreateBid.newCoinTrade(
                id = bidUuid.toString(),
                quote = newCoins(100, bidderDenom),
                base = newCoins(150, "nhash"),
            ),
            signer = BilateralAccounts.bidderAccount
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The quote should be withdrawn from the bidder's account",
        )
        bilateralClient.cancelBid(
            bidId = bidUuid.toString(),
            signer = BilateralAccounts.bidderAccount,
        )
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "After canceling a bid, the quote should be returned to the bidder",
        )
    }
}
