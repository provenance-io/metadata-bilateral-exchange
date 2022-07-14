package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.ScopeTradeAsk
import io.provenance.bilateral.execute.Bid.ScopeTradeBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.execute.UpdateAsk
import io.provenance.bilateral.execute.UpdateBid
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.metadata.v1.PartyType
import io.provenance.metadata.v1.ScopeRequest
import io.provenance.scope.util.MetadataAddress
import mu.KLogging
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.testGetScopeTrade
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertSingle
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import testconfiguration.util.ScopeWriteUtil
import java.time.OffsetDateTime
import java.util.UUID
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ScopeTradeIntTest : ContractIntTest() {
    private companion object : KLogging()

    @Test
    fun testScopeTradeFullFlow() {
        val (scopeUuid, _) = ScopeWriteUtil.writeMockScope(
            pbClient = pbClient,
            signer = BilateralAccounts.askerAccount,
            ownerAddress = contractInfo.contractAddress,
            valueOwnerAddress = contractInfo.contractAddress,
        )
        val bidderDenom = "scopetradefullflow"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(500, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk(
            ask = ScopeTradeAsk(
                id = askUuid.toString(),
                scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
                quote = newCoins(500, bidderDenom),
            ),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        logger.info("Creating scope trade ask [$askUuid]")
        val createAskResponse = bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        assertEquals(
            expected = askUuid.toString(),
            actual = createAskResponse.askId,
            message = "Expected the correct ask id to be returned with the create ask response",
        )
        assertEquals(
            expected = bilateralClient.getAsk(createAskResponse.askId),
            actual = createAskResponse.askOrder,
            message = "Expected the created ask order to be returned with the create ask response",
        )
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = ScopeTradeBid(
                id = bidUuid.toString(),
                scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
                quote = newCoins(500, bidderDenom),
            ),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        logger.info("Creating scope trade bid [$bidUuid]")
        val createBidResponse = bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        assertEquals(
            expected = bidUuid.toString(),
            actual = createBidResponse.bidId,
            message = "Expected the correct bid id to be returned with the create bid response",
        )
        assertEquals(
            expected = bilateralClient.getBid(createBidResponse.bidId),
            actual = createBidResponse.bidOrder,
            message = "Expected the created bid order to be returned with the create bid response",
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        val executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString())
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid]")
        val executeMatchResponse = bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        assertEquals(
            expected = askUuid.toString(),
            actual = executeMatchResponse.askId,
            message = "Expected the correct ask id to be returned with the execute match response",
        )
        assertEquals(
            expected = bidUuid.toString(),
            actual = executeMatchResponse.bidId,
            message = "Expected the correct bid id to be returned with the execute match response",
        )
        assertTrue(
            actual = executeMatchResponse.askDeleted,
            message = "Expected the execute match response to indicate that the ask was deleted",
        )
        assertTrue(
            actual = executeMatchResponse.bidDeleted,
            message = "Expected the execute match response to indicate that the bid was deleted",
        )
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 500L,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), bidderDenom),
            message = "The asker should have received the bidder's [$bidderDenom] after the trade was completed",
        )
        val scopeInfo = pbClient.metadataClient.scope(ScopeRequest.newBuilder().setScopeId(scopeUuid.toString()).build())
        assertEquals(
            expected = BilateralAccounts.bidderAccount.address(),
            actual = scopeInfo.scope.scope.ownersList.assertSingle("Expected a single owner to be set on the scope") { party ->
                party.role == PartyType.PARTY_TYPE_OWNER
            }.address,
            message = "The bidder to now be marked as the only owner on the scope",
        )
        assertEquals(
            expected = BilateralAccounts.bidderAccount.address(),
            actual = scopeInfo.scope.scope.valueOwnerAddress,
            message = "Expected the bidder to now be the value owner of the scope",
        )
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The bidder should no longer have any of its test denom [$bidderDenom]",
        )
    }

    @Test
    fun testCancelAsk() {
        val (scopeUuid, _) = ScopeWriteUtil.writeMockScope(
            pbClient = pbClient,
            signer = BilateralAccounts.askerAccount,
            ownerAddress = contractInfo.contractAddress,
            valueOwnerAddress = contractInfo.contractAddress,
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk(
            ask = ScopeTradeAsk(
                id = askUuid.toString(),
                scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
                quote = newCoins(5000, "nhash"),
            ),
        )
        bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        val askOrder = bilateralClient.assertAskExists(askUuid.toString())
        val response = bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "Expected the correct ask id to be included in the cancel ask response",
        )
        assertEquals(
            expected = askOrder,
            actual = response.cancelledAskOrder,
            message = "Expected the cancelled ask order to be included in the cancel ask response",
        )
        val scopeInfo = pbClient.metadataClient.scope(ScopeRequest.newBuilder().setScopeId(scopeUuid.toString()).build())
        assertEquals(
            expected = BilateralAccounts.askerAccount.address(),
            actual = scopeInfo.scope.scope.ownersList.assertSingle("Expected a single owner to be set on the scope") { party ->
                party.role == PartyType.PARTY_TYPE_OWNER
            }.address,
            message = "The asker to now be marked as the only owner on the scope after the ask's cancellation",
        )
        assertEquals(
            expected = BilateralAccounts.askerAccount.address(),
            actual = scopeInfo.scope.scope.valueOwnerAddress,
            message = "Expected the asker to now be the value owner of the scope after the ask's cancellatiion",
        )
    }

    @Test
    fun testCancelBid() {
        val bidderDenom = "testscopetradecancelbid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(10000, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = ScopeTradeBid(
                id = bidUuid.toString(),
                scopeAddress = MetadataAddress.forScope(UUID.randomUUID()).toString(),
                quote = newCoins(10000, bidderDenom),
            ),
        )
        bilateralClient.createBid(createBid, BilateralAccounts.bidderAccount)
        val bidOrder = bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "Expected the bidder's [$bidderDenom] coin to be held in escrow when the bid is created",
        )
        val response = bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        assertEquals(
            expected = bidUuid.toString(),
            actual = response.bidId,
            message = "Expected the correct bid id to be included in the cancel bid response",
        )
        assertEquals(
            expected = bidOrder,
            actual = response.cancelledBidOrder,
            message = "Expected the cancelled bid order to be included in the cancel bid response",
        )
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 10000L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The bidder's entire [$bidderDenom] quote balance should be returned when the bid is cancelled",
        )
    }

    @Test
    fun testUpdateAsk() {
        val (scopeUuid, _) = ScopeWriteUtil.writeMockScope(
            pbClient = pbClient,
            signer = BilateralAccounts.askerAccount,
            ownerAddress = contractInfo.contractAddress,
            valueOwnerAddress = contractInfo.contractAddress,
        )
        val quoteDenom = "updatescopetradeaskquote"
        val askUuid = UUID.randomUUID()
        assertSucceeds("Expected creating a scope trade ask to succeed") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = ScopeTradeAsk(
                        id = askUuid.toString(),
                        scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
                        quote = newCoins(500, quoteDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        val response = assertSucceeds("Expected updating a scope trade ask to succeed") {
            bilateralClient.updateAsk(
                updateAsk = UpdateAsk(
                    ask = ScopeTradeAsk(
                        id = askUuid.toString(),
                        scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
                        quote = newCoins(1200, quoteDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "Expected the correct ask id to be returned with the update ask response",
        )
        assertEquals(
            expected = bilateralClient.getAsk(response.askId),
            actual = response.updatedAskOrder,
            message = "Expected the updated ask order to be included with the update ask response",
        )
        assertEquals(
            expected = newCoins(1200, quoteDenom),
            actual = response.updatedAskOrder.testGetScopeTrade().quote,
            message = "Expected the updated ask order to include the new quote",
        )
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
    }

    @Test
    fun testUpdateBid() {
        val quoteDenom = "updatescopetradebidquote"
        val quoteDenom2 = "updatescopetradebidquote2"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, quoteDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, quoteDenom2),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val bidUuid = UUID.randomUUID()
        val scopeAddress = MetadataAddress.forScope(UUID.randomUUID()).toString()
        assertSucceeds("Expected creating a scope trade bid to succeed") {
            bilateralClient.createBid(
                createBid = CreateBid(
                    bid = ScopeTradeBid(
                        id = bidUuid.toString(),
                        scopeAddress = scopeAddress,
                        quote = newCoins(100, quoteDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 900,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The bid should have removed the correct amount of quote funds from the bidder account",
        )
        val response = assertSucceeds("Expected updating a scope trade bid to succeed") {
            bilateralClient.updateBid(
                updateBid = UpdateBid(
                    bid = ScopeTradeBid(
                        id = bidUuid.toString(),
                        scopeAddress = scopeAddress,
                        quote = newCoins(300, quoteDenom2),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = bidUuid.toString(),
            actual = response.bidId,
            message = "Expected the correct bid id to be returned with the update bid response",
        )
        assertEquals(
            expected = bilateralClient.getBid(response.bidId),
            actual = response.updatedBidOrder,
            message = "Expected the updated ask order to be included with the update ask response",
        )
        assertEquals(
            expected = newCoins(300, quoteDenom2),
            actual = response.updatedBidOrder.testGetScopeTrade().quote,
            message = "Expected the updated bid order to include the new quote",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The original bid quote denom should be refunded to the bidder",
        )
        assertEquals(
            expected = 700,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom2),
            message = "The bid should have debited the new quote denom from the bidder",
        )
        bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }
}
