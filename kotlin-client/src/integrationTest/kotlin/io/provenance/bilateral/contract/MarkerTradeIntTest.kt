package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.marker.v1.Access
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.getMarkerAccount
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
        val createAsk = CreateAsk.newMarkerTrade(
            id = askUuid.toString(),
            markerDenom = markerDenom,
            quotePerShare = newCoins(15, bidderDenom),
            descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
        )
        bilateralClient.createAsk(createAsk = createAsk, signer = BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        assertTrue(
            actual = pbClient
                .getMarkerAccount(markerDenom)
                .accessControlList
                .none { accessGrant -> accessGrant.address == BilateralAccounts.askerAccount.address() },
            message = "The contract should remove access for the asker from the marker after receiving it",
        )
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newMarkerTrade(
            id = bidUuid.toString(),
            markerDenom = markerDenom,
            quote = newCoins(150, bidderDenom),
            descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
        )
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        bilateralClient.executeMatch(
            executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString()),
            signer = BilateralAccounts.adminAccount,
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
        val createAsk = CreateAsk.newMarkerTrade(
            id = askUuid.toString(),
            markerDenom = markerDenom,
            quotePerShare = newCoins(10, "nhash"),
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
        bilateralClient.assertAskExists(askUuid.toString())
        assertFails("When the contract already has a marker held, a new ask should not be able to be posted for the same marker") {
            bilateralClient.createAsk(
                createAsk = CreateAsk.newMarkerTrade(
                    id = UUID.randomUUID().toString(),
                    markerDenom = markerDenom,
                    quotePerShare = newCoins(150, "nhash"),
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
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
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
        val createBid = CreateBid.newMarkerTrade(
            id = bidUuid.toString(),
            markerDenom = bidderDenom,
            quote = newCoins(99, bidderDenom),
        )
        bilateralClient.createBid(createBid, BilateralAccounts.bidderAccount)
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 1L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "Expected all the required bidder denom [$bidderDenom] to be held in contract escrow",
        )
        bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "Expected all the required bidder denom [$bidderDenom] to be returned to the bidder after cancelling the bid",
        )
    }
}
