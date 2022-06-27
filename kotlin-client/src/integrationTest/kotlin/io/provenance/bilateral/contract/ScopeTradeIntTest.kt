package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CancelAsk
import io.provenance.bilateral.execute.CancelBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.metadata.v1.PartyType
import io.provenance.metadata.v1.ScopeRequest
import io.provenance.scope.util.MetadataAddress
import mu.KLogging
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.getBalance
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertSingle
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import testconfiguration.util.ScopeWriteUtil
import java.time.OffsetDateTime
import java.util.UUID
import kotlin.test.assertEquals

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
        val createAsk = CreateAsk.newScopeTrade(
            id = askUuid.toString(),
            scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
            quote = newCoins(500, bidderDenom),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        logger.info("Creating scope trade ask [$askUuid]")
        bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newScopeTrade(
            id = bidUuid.toString(),
            scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
            quote = newCoins(500, bidderDenom),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        logger.info("Creating scope trade bid [$bidUuid]")
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        val executeMatch = ExecuteMatch.new(askUuid.toString(), bidUuid.toString())
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid]")
        bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
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
        val createAsk = CreateAsk.newScopeTrade(
            id = askUuid.toString(),
            scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
            quote = newCoins(5000, "nhash"),
        )
        bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        val cancelAsk = CancelAsk.new(askUuid.toString())
        bilateralClient.cancelAsk(cancelAsk, BilateralAccounts.askerAccount)
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
        val createBid = CreateBid.newScopeTrade(
            id = bidUuid.toString(),
            scopeAddress = MetadataAddress.forScope(UUID.randomUUID()).toString(),
            quote = newCoins(10000, bidderDenom),
        )
        bilateralClient.createBid(createBid, BilateralAccounts.bidderAccount)
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "Expected the bidder's [$bidderDenom] coin to be held in escrow when the bid is created",
        )
        val cancelBid = CancelBid.new(bidUuid.toString())
        bilateralClient.cancelBid(cancelBid, BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 10000L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The bidder's entire [$bidderDenom] quote balance should be returned when the bid is cancelled",
        )
    }
}
