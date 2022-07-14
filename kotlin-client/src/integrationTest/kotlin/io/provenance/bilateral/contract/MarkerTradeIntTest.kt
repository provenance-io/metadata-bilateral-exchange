package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.MarkerTradeAsk
import io.provenance.bilateral.execute.Bid.MarkerTradeBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.execute.UpdateAsk
import io.provenance.bilateral.execute.UpdateBid
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.marker.v1.Access
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.getMarkerAccount
import testconfiguration.extensions.testGetMarkerTrade
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertSingle
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.createMarker
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.grantMarkerAccess
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.time.OffsetDateTime
import java.util.UUID
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertTrue

class MarkerTradeIntTest : ContractIntTest() {
    @Test
    fun testSimpleMarkerTrade() {
        val markerDenom = "simplemarkertrade"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW, Access.ACCESS_BURN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = 10,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val bidderDenom = "simplemarkertradebid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(amount = 150, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk(
            ask = MarkerTradeAsk(
                id = askUuid.toString(),
                markerDenom = markerDenom,
                quotePerShare = newCoins(15, bidderDenom),
            ),
            descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
        )
        val createAskResponse = bilateralClient.createAsk(createAsk = createAsk, signer = BilateralAccounts.askerAccount)
        assertEquals(
            expected = askUuid.toString(),
            actual = createAskResponse.askId,
            message = "Expected the correct ask id to be returned in the create ask response",
        )
        assertEquals(
            expected = bilateralClient.getAsk(askUuid.toString()),
            actual = createAskResponse.askOrder,
            message = "Expected the created ask order to be returned in the create ask response",
        )
        bilateralClient.assertAskExists(askUuid.toString())
        assertTrue(
            actual = pbClient
                .getMarkerAccount(markerDenom)
                .accessControlList
                .none { accessGrant -> accessGrant.address == BilateralAccounts.askerAccount.address() },
            message = "The contract should remove access for the asker from the marker after receiving it",
        )
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = MarkerTradeBid(
                id = bidUuid.toString(),
                markerDenom = markerDenom,
                quote = newCoins(150, bidderDenom),
            ),
            descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
        )
        val createBidResponse = bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        assertEquals(
            expected = bidUuid.toString(),
            actual = createBidResponse.bidId,
            message = "Expected the correct bid id to be returned in the create bid response",
        )
        assertEquals(
            expected = bilateralClient.getBid(createBidResponse.bidId),
            actual = createBidResponse.bidOrder,
            message = "Expected the created bid order to be returned in the create bid response",
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        val executeMatchResponse = bilateralClient.executeMatch(
            executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString()),
            signer = BilateralAccounts.adminAccount,
        )
        assertEquals(
            expected = askUuid.toString(),
            actual = executeMatchResponse.askId,
            message = "Expected the correct ask id to be returned in the execute match response",
        )
        assertEquals(
            expected = bidUuid.toString(),
            actual = executeMatchResponse.bidId,
            message = "Expected the correct bid id to be returned in the execute match response",
        )
        assertTrue(
            actual = executeMatchResponse.askDeleted,
            message = "Expected the match response to indicate that the ask was deleted",
        )
        assertTrue(
            actual = executeMatchResponse.bidDeleted,
            message = "Expected the match response to indicate that the bid was deleted",
        )
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 150L,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), bidderDenom),
            message = "The asker should have received the entirety of the bidder's denom in exchange for the scope",
        )
        val access = pbClient
            .getMarkerAccount(markerDenom)
            .accessControlList
            .assertSingle("There should only be a single access on the marker after completing the trade")
        assertEquals(
            expected = BilateralAccounts.bidderAccount.address(),
            actual = access.address,
            message = "The bidder should be the sole permissioned entity on the marker after the trade completes",
        )
        assertEquals(
            expected = markerPermissions.sorted(),
            actual = access.permissionsList.sorted(),
            message = "The bidder should be granted identical permissions to the asker after the trade completes",
        )
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "After the trade is made, the bidder should no longer have any of its [$bidderDenom]",
        )
    }

    @Test
    fun testCancelAsk() {
        val markerDenom = "cancelaskmarkertrade"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_DEPOSIT)
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = 10L,
            permissions = markerPermissions,
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk(
            ask = MarkerTradeAsk(
                id = askUuid.toString(),
                markerDenom = markerDenom,
                quotePerShare = newCoins(10, "nhash"),
            ),
        )
        assertFails("When the contract is not an admin on the marker, creating the ask should fail") {
            bilateralClient.createAsk(createAsk = createAsk, signer = BilateralAccounts.askerAccount)
        }
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        assertSucceeds("Now that the contract has admin access on the marker, creating the ask should succeed") {
            bilateralClient.createAsk(createAsk = createAsk, signer = BilateralAccounts.askerAccount)
        }
        val askOrder = bilateralClient.assertAskExists(askUuid.toString())
        assertFails("When the contract already has a marker held, a new ask should not be able to be posted for the same marker") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = MarkerTradeAsk(
                        id = UUID.randomUUID().toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(150, "nhash"),
                    ),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        assertTrue(
            actual = pbClient.getMarkerAccount(markerDenom)
                .accessControlList
                .none { accessGrant -> accessGrant.address == BilateralAccounts.askerAccount.address() },
            message = "The contract should remove access for the asker from the marker after receiving it",
        )
        val response = bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "Expected the correct ask id to be included in the response",
        )
        assertEquals(
            expected = askOrder,
            actual = response.cancelledAskOrder,
            message = "Expected the cancelled ask order to be included in the response",
        )
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        val grant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single access control should exist on the marker after cancelling the ask")
        assertEquals(
            expected = BilateralAccounts.askerAccount.address(),
            actual = grant.address,
            message = "The asker account should be the permissioned account on the marker after cancelling the ask",
        )
        assertEquals(
            expected = markerPermissions.sorted(),
            actual = grant.permissionsList.sorted(),
            message = "The asker's original permissions should be returned in totality after cancelling the ask",
        )
    }

    @Test
    fun testCancelBid() {
        val bidderDenom = "markertradecancelbid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(100, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = MarkerTradeBid(
                id = bidUuid.toString(),
                markerDenom = bidderDenom,
                quote = newCoins(99, bidderDenom),
            ),
        )
        bilateralClient.createBid(createBid, BilateralAccounts.bidderAccount)
        val bidOrder = bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 1L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "Expected all the required bidder denom [$bidderDenom] to be held in contract escrow",
        )
        val response = bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        assertEquals(
            expected = bidUuid.toString(),
            actual = response.bidId,
            message = "Expected the correct bid id to be included in the response",
        )
        assertEquals(
            expected = bidOrder,
            actual = response.cancelledBidOrder,
            message = "Expected the cancelled bid order to be included in the response",
        )
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "Expected all the required bidder denom [$bidderDenom] to be returned to the bidder after cancelling the bid",
        )
    }

    @Test
    fun testUpdateAsk() {
        val markerDenom = "updatemarkertradeask"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW, Access.ACCESS_BURN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = 10,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val quoteDenom = "updatemarkertradeaskquote"
        val askUuid = UUID.randomUUID()
        assertSucceeds("Creating an ask should succeed") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = MarkerTradeAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(15, quoteDenom),
                    ),
                    descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        val response = assertSucceeds("Updating the ask should succeed") {
            bilateralClient.updateAsk(
                updateAsk = UpdateAsk(
                    ask = MarkerTradeAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(100, quoteDenom),
                    ),
                    descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "Expected the update ask response to contain the correct ask id",
        )
        assertEquals(
            expected = bilateralClient.getAsk(response.askId),
            actual = response.updatedAskOrder,
            message = "Expected the updated ask order to be included in the response",
        )
        assertEquals(
            expected = newCoins(100, quoteDenom),
            actual = response.updatedAskOrder.testGetMarkerTrade().quotePerShare,
            message = "The quote per share in the ask order should be properly updated",
        )
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
    }

    @Test
    fun testUpdateBid() {
        val markerDenom = "updatemarkertradebid"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW, Access.ACCESS_BURN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = 10,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val quoteDenom = "updatemarkertradebidquote"
        val quoteDenom2 = "updatemarkertradebidquote2"
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
        val askUuid = UUID.randomUUID()
        assertSucceeds("Creating an ask should succeed") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = MarkerTradeAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(15, quoteDenom),
                    ),
                    descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        assertSucceeds("Creating a bid should succeed") {
            bilateralClient.createBid(
                createBid = CreateBid(
                    bid = MarkerTradeBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        quote = newCoins(100, quoteDenom),
                    )
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 900,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The bidder's account should have properly been debited of its quote funds",
        )
        val response = assertSucceeds("Updating a bid should succeed") {
            bilateralClient.updateBid(
                updateBid = UpdateBid(
                    bid = MarkerTradeBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        quote = newCoins(700, quoteDenom2),
                    )
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        assertEquals(
            expected = bidUuid.toString(),
            actual = response.bidId,
            message = "The correct bid id should be included in the update bid response",
        )
        assertEquals(
            expected = bilateralClient.getBid(response.bidId),
            actual = response.updatedBidOrder,
            message = "The updated bid order should be included in the update bid response",
        )
        assertEquals(
            expected = newCoins(700, quoteDenom2),
            actual = response.updatedBidOrder.testGetMarkerTrade().quote,
            message = "The quote should be properly updated in the bid order",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The original bid's quote should be fully refunded to the bidder",
        )
        assertEquals(
            expected = 300,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom2),
            message = "The new quote balance should be properly debited from the bidder's account",
        )
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }
}
