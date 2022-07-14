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
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.getMarkerAccount
import testconfiguration.extensions.testGetMarkerTrade
import testconfiguration.functions.assertSingle
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.createMarker
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.grantMarkerAccess
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
import testconfiguration.ContractIntTest
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
            ownerAccount = asker,
            denomName = markerDenom,
            supply = 10,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val bidderDenom = "simplemarkertradebid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(amount = 150, bidderDenom),
            receiverAddress = bidder.address(),
        )
        val askUuid = UUID.randomUUID()
        createAsk(
            createAsk = CreateAsk(
                ask = MarkerTradeAsk(
                    id = askUuid.toString(),
                    markerDenom = markerDenom,
                    quotePerShare = newCoins(15, bidderDenom),
                ),
                descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
            )
        )
        assertTrue(
            actual = pbClient
                .getMarkerAccount(markerDenom)
                .accessControlList
                .none { accessGrant -> accessGrant.address == asker.address() },
            message = "The contract should remove access for the asker from the marker after receiving it",
        )
        val bidUuid = UUID.randomUUID()
        createBid(
            createBid = CreateBid(
                bid = MarkerTradeBid(
                    id = bidUuid.toString(),
                    markerDenom = markerDenom,
                    quote = newCoins(150, bidderDenom),
                ),
                descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
            ),
        )
        val executeMatchResponse = executeMatch(
            executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString()),
        )
        assertTrue(
            actual = executeMatchResponse.askDeleted,
            message = "Expected the match response to indicate that the ask was deleted",
        )
        assertTrue(
            actual = executeMatchResponse.bidDeleted,
            message = "Expected the match response to indicate that the bid was deleted",
        )
        assertEquals(
            expected = 150L,
            actual = pbClient.getBalance(asker.address(), bidderDenom),
            message = "The asker should have received the entirety of the bidder's denom in exchange for the scope",
        )
        val access = pbClient
            .getMarkerAccount(markerDenom)
            .accessControlList
            .assertSingle("There should only be a single access on the marker after completing the trade")
        assertEquals(
            expected = bidder.address(),
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
            actual = pbClient.getBalance(bidder.address(), bidderDenom),
            message = "After the trade is made, the bidder should no longer have any of its [$bidderDenom]",
        )
    }

    @Test
    fun testCancelAsk() {
        val markerDenom = "cancelaskmarkertrade"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_DEPOSIT)
        createMarker(
            pbClient = pbClient,
            ownerAccount = asker,
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
            createAsk(createAsk = createAsk)
        }
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val createResponse = assertSucceeds("Now that the contract has admin access on the marker, creating the ask should succeed") {
            createAsk(createAsk = createAsk)
        }
        assertFails("When the contract already has a marker held, a new ask should not be able to be posted for the same marker") {
            createAsk(
                createAsk = CreateAsk(
                    ask = MarkerTradeAsk(
                        id = UUID.randomUUID().toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(150, "nhash"),
                    ),
                ),
            )
        }
        assertTrue(
            actual = pbClient.getMarkerAccount(markerDenom)
                .accessControlList
                .none { accessGrant -> accessGrant.address == asker.address() },
            message = "The contract should remove access for the asker from the marker after receiving it",
        )
        val cancelResponse = cancelAsk(askUuid.toString(), asker)
        assertEquals(
            expected = createResponse.askOrder,
            actual = cancelResponse.cancelledAskOrder,
            message = "Expected the cancelled ask order to be included in the response",
        )
        val grant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single access control should exist on the marker after cancelling the ask")
        assertEquals(
            expected = asker.address(),
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
            receiverAddress = bidder.address(),
        )
        val bidUuid = UUID.randomUUID()
        val createResponse = createBid(
            createBid = CreateBid(
                bid = MarkerTradeBid(
                    id = bidUuid.toString(),
                    markerDenom = bidderDenom,
                    quote = newCoins(99, bidderDenom),
                ),
            )
        )
        assertEquals(
            expected = 1L,
            actual = pbClient.getBalance(bidder.address(), bidderDenom),
            message = "Expected all the required bidder denom [$bidderDenom] to be held in contract escrow",
        )
        val cancelResponse = cancelBid(bidUuid.toString(), bidder)
        assertEquals(
            expected = createResponse.bidOrder,
            actual = cancelResponse.cancelledBidOrder,
            message = "Expected the cancelled bid order to be included in the response",
        )
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(bidder.address(), bidderDenom),
            message = "Expected all the required bidder denom [$bidderDenom] to be returned to the bidder after cancelling the bid",
        )
    }

    @Test
    fun testUpdateAsk() {
        val markerDenom = "updatemarkertradeask"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW, Access.ACCESS_BURN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = asker,
            denomName = markerDenom,
            supply = 10,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val quoteDenom = "updatemarkertradeaskquote"
        val askUuid = UUID.randomUUID()
        assertSucceeds("Creating an ask should succeed") {
            createAsk(
                createAsk = CreateAsk(
                    ask = MarkerTradeAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(15, quoteDenom),
                    ),
                    descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
                ),
            )
        }
        val response = assertSucceeds("Updating the ask should succeed") {
            updateAsk(
                updateAsk = UpdateAsk(
                    ask = MarkerTradeAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(100, quoteDenom),
                    ),
                    descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
                ),
            )
        }
        assertEquals(
            expected = newCoins(100, quoteDenom),
            actual = response.updatedAskOrder.testGetMarkerTrade().quotePerShare,
            message = "The quote per share in the ask order should be properly updated",
        )
    }

    @Test
    fun testUpdateBid() {
        val markerDenom = "updatemarkertradebid"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW, Access.ACCESS_BURN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = asker,
            denomName = markerDenom,
            supply = 10,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val quoteDenom = "updatemarkertradebidquote"
        val quoteDenom2 = "updatemarkertradebidquote2"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, quoteDenom),
            receiverAddress = bidder.address(),
        )
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(1000, quoteDenom2),
            receiverAddress = bidder.address(),
        )
        val askUuid = UUID.randomUUID()
        assertSucceeds("Creating an ask should succeed") {
            createAsk(
                createAsk = CreateAsk(
                    ask = MarkerTradeAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(15, quoteDenom),
                    ),
                    descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
                ),
            )
        }
        val bidUuid = UUID.randomUUID()
        assertSucceeds("Creating a bid should succeed") {
            createBid(
                createBid = CreateBid(
                    bid = MarkerTradeBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        quote = newCoins(100, quoteDenom),
                    )
                ),
                signer = bidder,
            )
        }
        assertEquals(
            expected = 900,
            actual = pbClient.getBalance(bidder.address(), quoteDenom),
            message = "The bidder's account should have properly been debited of its quote funds",
        )
        val response = assertSucceeds("Updating a bid should succeed") {
            updateBid(
                updateBid = UpdateBid(
                    bid = MarkerTradeBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        quote = newCoins(700, quoteDenom2),
                    )
                ),
                signer = bidder,
            )
        }
        assertEquals(
            expected = newCoins(700, quoteDenom2),
            actual = response.updatedBidOrder.testGetMarkerTrade().quote,
            message = "The quote should be properly updated in the bid order",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(bidder.address(), quoteDenom),
            message = "The original bid's quote should be fully refunded to the bidder",
        )
        assertEquals(
            expected = 300,
            actual = pbClient.getBalance(bidder.address(), quoteDenom2),
            message = "The new quote balance should be properly debited from the bidder's account",
        )
    }
}
