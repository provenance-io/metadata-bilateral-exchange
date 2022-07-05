package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.bilateral.models.ShareSaleType
import io.provenance.marker.v1.Access
import mu.KLogging
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

class MarkerShareSaleIntTest : ContractIntTest() {
    private companion object : KLogging()

    @Test
    fun testSingleTxShareSale() {
        val markerDenom = "singletx"
        val shareCount = 100L
        val shareSaleAmount = 50L
        val markerPermissions = listOf(Access.ACCESS_ADMIN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val bidderDenom = "singletxbid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(50, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newMarkerShareSale(
            id = askUuid.toString(),
            denom = markerDenom,
            quotePerShare = newCoins(1, bidderDenom),
            shareSaleType = ShareSaleType.single(shareSaleAmount.toString()),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
        )
        bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = contractInfo.contractAddress,
            actual = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single account should have marker permissions once the ask is created").address,
            message = "The contract should control the marker account after creating a marker share sale",
        )
        assertFails("An ask the for same marker should not be allowed once a marker share sale is created") {
            bilateralClient.createAsk(
                createAsk = CreateAsk.newMarkerShareSale(
                    id = UUID.randomUUID().toString(),
                    denom = markerDenom,
                    quotePerShare = newCoins(1, bidderDenom),
                    shareSaleType = ShareSaleType.single(shareSaleAmount.toString()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newMarkerShareSale(
            id = bidUuid.toString(),
            denom = markerDenom,
            shareCount = shareSaleAmount.toString(),
            quote = newCoins(50, bidderDenom),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
        )
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The bidder's denom [$bidderDenom] should be held in escrow when the bid is created",
        )
        val executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString())
        bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 50L,
            actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), bidderDenom),
            message = "The correct amount of [$bidderDenom] denom should be transferred to the asker",
        )
        val grant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single access should remain on the marker after executing the trade")
        assertEquals(
            expected = BilateralAccounts.askerAccount.address(),
            actual = grant.address,
            message = "The only access on the marker after the trade should be for the asker",
        )
        assertEquals(
            expected = markerPermissions.sorted(),
            actual = grant.permissionsList.sorted(),
            message = "The correct permissions should be returned to the asker after the sale completes",
        )
        assertEquals(
            expected = 50L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), markerDenom),
            message = "The correct amount of marker denom [$markerDenom] should be transferred to the bidder",
        )
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The correct amount of bidder denom [$bidderDenom] should be removed from the bidder's account",
        )
    }

    @Test
    fun testMultipleTxShareSale() {
        val markerDenom = "multipletx"
        val shareCount = 100L
        val sharePurchaseCount = 25L
        val shareCutoff = 25L
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_DEPOSIT, Access.ACCESS_DELETE)
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val bidderDenom = "multipletxbid"
        var expectedBidderDenomHoldings = 75L
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(expectedBidderDenomHoldings, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newMarkerShareSale(
            id = askUuid.toString(),
            denom = markerDenom,
            quotePerShare = newCoins(1, bidderDenom),
            shareSaleType = ShareSaleType.multiple(removeSaleShareThreshold = shareCutoff.toString()),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
        )
        bilateralClient.createAsk(
            createAsk = createAsk,
            signer = BilateralAccounts.askerAccount,
        )
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = contractInfo.contractAddress,
            actual = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Expected only a single access control list to exist after creating a share sale").address,
            message = "The contract should be the sole owner of the marker during the share sale",
        )
        val maxIteration = (shareCount - shareCutoff) / sharePurchaseCount - 1
        var expectedBidderMarkerHoldings = 0L
        var expectedAskerDenomHoldings = 0L
        for (counter in 0..maxIteration) {
            val bidUuid = UUID.randomUUID()
            val createBid = CreateBid.newMarkerShareSale(
                id = bidUuid.toString(),
                denom = markerDenom,
                shareCount = sharePurchaseCount.toString(),
                // Pay 1 bidderDenom per share
                quote = newCoins(sharePurchaseCount, bidderDenom),
                descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
            )
            bilateralClient.createBid(
                createBid = createBid,
                signer = BilateralAccounts.bidderAccount,
            )
            bilateralClient.assertBidExists(bidUuid.toString())
            expectedBidderDenomHoldings -= sharePurchaseCount
            assertEquals(
                expected = expectedBidderDenomHoldings,
                actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
                message = "Expected the proper amount of denom [$bidderDenom] to be taken from the bidder as the quote",
            )
            val executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString())
            bilateralClient.executeMatch(executeMatch = executeMatch, signer = BilateralAccounts.adminAccount)
            bilateralClient.assertBidIsDeleted(bidUuid.toString())
            expectedBidderMarkerHoldings += sharePurchaseCount
            expectedAskerDenomHoldings += sharePurchaseCount
            assertEquals(
                expected = expectedBidderMarkerHoldings,
                actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), markerDenom),
                message = "Expected the bidder to hold the correct amount of the marker denom [$markerDenom]"
            )
            assertEquals(
                expected = expectedAskerDenomHoldings,
                actual = pbClient.getBalance(BilateralAccounts.askerAccount.address(), bidderDenom),
                message = "Expected the asker to have received the correct amount of test denom [$bidderDenom]",
            )
            if (counter != maxIteration) {
                bilateralClient.assertAskExists(askUuid.toString(), "Ask should still exist because the share threshold has not yet been reached")
                assertEquals(
                    expected = contractInfo.contractAddress,
                    actual = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("A single access grant should exist on the marker").address,
                    message = "Expected the contract to still hold access to the marker after the sale is completed and the threshold has not yet been hit",
                )
            }
        }
        bilateralClient.assertAskIsDeleted(askUuid.toString(), "Ask should be deleted because the share sale threshold has been reached")
        val grant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("A single access grant should exist on the marker")
        assertEquals(
            expected = BilateralAccounts.askerAccount.address(),
            actual = grant.address,
            message = "Expected the asker to receive the marker after the sale has been completed",
        )
        assertEquals(
            expected = markerPermissions.sorted(),
            actual = grant.permissionsList.sorted(),
            message = "Expected all permissions to be properly restored to the asker",
        )
    }

    @Test
    fun testCancelAsk() {
        val markerDenom = "sharesalecancelask"
        val markerPermissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_DEPOSIT, Access.ACCESS_BURN, Access.ACCESS_MINT)
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = 100L,
            permissions = markerPermissions,
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newMarkerShareSale(
            id = askUuid.toString(),
            denom = markerDenom,
            quotePerShare = newCoins(100, "nhash"),
            shareSaleType = ShareSaleType.single(100.toString()),
        )
        assertFails("An ask cannot be created when the contract does not have admin and withdraw permissions on the marker") {
            bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        }
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN),
        )
        assertFails("An ask cannot bbe created when the contract only has admin permissions. It also needs withdraw") {
            bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        }
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        assertSucceeds("An ask should be allowed when the marker has the correct permission structure") {
            bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        }
        bilateralClient.assertAskExists(askUuid.toString())
        val beforeCancelGrant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single permission should exist in the marker after successfully creating an ask")
        assertEquals(
            expected = contractInfo.contractAddress,
            actual = beforeCancelGrant.address,
            message = "The contract should be permissioned on the marker after an ask is created with it",
        )
        assertEquals(
            expected = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW).sorted(),
            actual = beforeCancelGrant.permissionsList.sorted(),
            message = "The contract's permissions should not be modified after the marker has been escrowed into the contract",
        )
        assertFails("A new ask for the same marker cannot be created while the marker is already held in the contract") {
            bilateralClient.createAsk(
                createAsk = CreateAsk.newMarkerShareSale(
                    id = UUID.randomUUID().toString(),
                    denom = markerDenom,
                    quotePerShare = newCoins(1, "nhash"),
                    shareSaleType = ShareSaleType.single(100.toString()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        val afterCancelGrant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single permission should exist on the marker after cancelling the ask")
        assertEquals(
            expected = BilateralAccounts.askerAccount.address(),
            actual = afterCancelGrant.address,
            message = "After cancelling the ask, the asker should regain admin control over the marker",
        )
        assertEquals(
            expected = markerPermissions.sorted(),
            actual = afterCancelGrant.permissionsList.sorted(),
            message = "After cancelling the ask, the asker should regain its exact permissions",
        )
    }

    @Test
    fun testCancelBid() {
        val bidderDenom = "cancelmarkersharebid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(200, bidderDenom),
            receiverAddress = BilateralAccounts.bidderAccount.address(),
            sendAmount = 100L,
        )
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newMarkerShareSale(
            id = bidUuid.toString(),
            denom = bidderDenom,
            shareCount = 100.toString(),
            quote = newCoins(100, bidderDenom),
        )
        bilateralClient.createBid(createBid, BilateralAccounts.bidderAccount)
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "All bidder denom [$bidderDenom] should be held in contract escrow after creating the bid",
        )
        bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "All bidder denom [$bidderDenom] should be returned to the bidder after cancelling the bid",
        )
    }
}
