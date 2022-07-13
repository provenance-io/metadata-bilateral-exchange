package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.MarkerShareSaleAsk
import io.provenance.bilateral.execute.Bid.MarkerShareSaleBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.execute.UpdateAsk
import io.provenance.bilateral.execute.UpdateBid
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.bilateral.models.ShareSaleType
import io.provenance.marker.v1.Access
import mu.KLogging
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.getMarkerAccount
import testconfiguration.extensions.testGetMarkerShareSale
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
import java.math.BigInteger
import java.time.OffsetDateTime
import java.util.UUID
import kotlin.test.assertEquals
import kotlin.test.assertFails
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class MarkerShareSaleIntTest : ContractIntTest() {
    private companion object : KLogging()

    @Test
    fun testSingleTxShareSale() {
        val markerDenom = "singletx"
        val shareCount = 100L
        val shareSaleAmount = 50.toBigInteger()
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
        val createAsk = CreateAsk(
            ask = MarkerShareSaleAsk(
                id = askUuid.toString(),
                markerDenom = markerDenom,
                sharesToSell = shareSaleAmount,
                quotePerShare = newCoins(1, bidderDenom),
                shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
            ),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        val createAskResponse = bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = askUuid.toString(),
            actual = createAskResponse.askId,
            message = "The create ask response should include the correct ask id",
        )
        assertEquals(
            expected = bilateralClient.getAsk(createAskResponse.askId),
            actual = createAskResponse.askOrder,
            message = "The create ask response should include the created ask order",
        )
        assertEquals(
            expected = contractInfo.contractAddress,
            actual = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single account should have marker permissions once the ask is created").address,
            message = "The contract should control the marker account after creating a marker share sale",
        )
        assertFails("An ask the for same marker should not be allowed once a marker share sale is created") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = MarkerShareSaleAsk(
                        id = UUID.randomUUID().toString(),
                        markerDenom = markerDenom,
                        sharesToSell = shareSaleAmount,
                        quotePerShare = newCoins(1, bidderDenom),
                        shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                    ),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = MarkerShareSaleBid(
                id = bidUuid.toString(),
                markerDenom = markerDenom,
                shareCount = shareSaleAmount,
                quote = newCoins(50, bidderDenom),
            ),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        val createBidResponse = bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = bidUuid.toString(),
            actual = createBidResponse.bidId,
            message = "The create bid response should include the correct bid id",
        )
        assertEquals(
            expected = bilateralClient.getBid(bidUuid.toString()),
            actual = createBidResponse.bidOrder,
            message = "The create bid response should include the created bid order",
        )
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "The bidder's denom [$bidderDenom] should be held in escrow when the bid is created",
        )
        val executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString())
        val executeMatchResponse = bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        assertEquals(
            expected = askUuid.toString(),
            actual = executeMatchResponse.askId,
            message = "The execute match response should include the proper ask id",
        )
        assertEquals(
            expected = bidUuid.toString(),
            actual = executeMatchResponse.bidId,
            message = "The execute match response should include the proper bid id",
        )
        assertTrue(
            actual = executeMatchResponse.askDeleted,
            message = "The response should indicate that the ask was deleted",
        )
        assertTrue(
            actual = executeMatchResponse.bidDeleted,
            message = "The response should indicate that the bid was deleted",
        )
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
        val sharePurchaseCount = 25.toBigInteger()
        val sharesToSell = 75.toBigInteger()
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
        val createAsk = CreateAsk(
            ask = MarkerShareSaleAsk(
                id = askUuid.toString(),
                markerDenom = markerDenom,
                sharesToSell = sharesToSell,
                quotePerShare = newCoins(1, bidderDenom),
                shareSaleType = ShareSaleType.MULTIPLE_TRANSACTIONS,
            ),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        val createAskResponse = bilateralClient.createAsk(
            createAsk = createAsk,
            signer = BilateralAccounts.askerAccount,
        )
        assertEquals(
            expected = askUuid.toString(),
            actual = createAskResponse.askId,
            message = "The create ask response should include the correct ask id",
        )
        assertEquals(
            expected = bilateralClient.getAsk(createAskResponse.askId),
            actual = createAskResponse.askOrder,
            message = "The create ask response should include the created ask order",
        )
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = contractInfo.contractAddress,
            actual = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Expected only a single access control list to exist after creating a share sale").address,
            message = "The contract should be the sole owner of the marker during the share sale",
        )
        val maxIteration = (sharesToSell / sharePurchaseCount - BigInteger.ONE).toLong()
        var expectedBidderMarkerHoldings = 0L
        var expectedAskerDenomHoldings = 0L
        for (counter in 0..maxIteration) {
            val bidUuid = UUID.randomUUID()
            val createBid = CreateBid(
                bid = MarkerShareSaleBid(
                    id = bidUuid.toString(),
                    markerDenom = markerDenom,
                    shareCount = sharePurchaseCount,
                    // Pay 1 bidderDenom per share
                    quote = newCoins(sharePurchaseCount.toLong(), bidderDenom),
                ),
                descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
            )
            val createBidResponse = bilateralClient.createBid(
                createBid = createBid,
                signer = BilateralAccounts.bidderAccount,
            )
            assertEquals(
                expected = bidUuid.toString(),
                actual = createBidResponse.bidId,
                message = "The create bid response should include the correct bid id",
            )
            assertEquals(
                expected = bilateralClient.getBid(createBidResponse.bidId),
                actual = createBidResponse.bidOrder,
                message = "The create bid response should include the created bid order",
            )
            bilateralClient.assertBidExists(bidUuid.toString())
            expectedBidderDenomHoldings -= sharePurchaseCount.toLong()
            assertEquals(
                expected = expectedBidderDenomHoldings,
                actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
                message = "Expected the proper amount of denom [$bidderDenom] to be taken from the bidder as the quote",
            )
            val executeMatch = ExecuteMatch(askUuid.toString(), bidUuid.toString())
            val executeMatchResponse = bilateralClient.executeMatch(executeMatch = executeMatch, signer = BilateralAccounts.adminAccount)
            bilateralClient.assertBidIsDeleted(bidUuid.toString())
            assertEquals(
                expected = askUuid.toString(),
                actual = executeMatchResponse.askId,
                message = "The execute match response should include the correct ask id",
            )
            assertEquals(
                expected = bidUuid.toString(),
                actual = executeMatchResponse.bidId,
                message = "The execute match response should include the correct bid id",
            )
            assertTrue(
                actual = executeMatchResponse.bidDeleted,
                message = "The execute match response should indicate that the bid was deleted",
            )
            expectedBidderMarkerHoldings += sharePurchaseCount.toLong()
            expectedAskerDenomHoldings += sharePurchaseCount.toLong()
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
                assertFalse(
                    actual = executeMatchResponse.askDeleted,
                    message = "The execute match response should indicate that the ask was not deleted because the sale is not over",
                )
            } else {
                assertTrue(
                    actual = executeMatchResponse.askDeleted,
                    message = "The execute match response should indicate that the ask was deleted because the sale is over",
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
        val createAsk = CreateAsk(
            ask = MarkerShareSaleAsk(
                id = askUuid.toString(),
                markerDenom = markerDenom,
                sharesToSell = 100.toBigInteger(),
                quotePerShare = newCoins(100, "nhash"),
                shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
            ),
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
        val askOrder = bilateralClient.assertAskExists(askUuid.toString())
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
                createAsk = CreateAsk(
                    ask = MarkerShareSaleAsk(
                        id = UUID.randomUUID().toString(),
                        markerDenom = markerDenom,
                        sharesToSell = 100.toBigInteger(),
                        quotePerShare = newCoins(1, "nhash"),
                        shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                    ),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        val response = bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "The correct ask id should be included in the response",
        )
        assertEquals(
            expected = askOrder,
            actual = response.cancelledAskOrder,
            message = "The cancelled ask order should be included in the response",
        )
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
        val createBid = CreateBid(
            bid = MarkerShareSaleBid(
                id = bidUuid.toString(),
                markerDenom = bidderDenom,
                shareCount = 100.toBigInteger(),
                quote = newCoins(100, bidderDenom),
            ),
        )
        bilateralClient.createBid(createBid, BilateralAccounts.bidderAccount)
        val bidOrder = bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "All bidder denom [$bidderDenom] should be held in contract escrow after creating the bid",
        )
        val response = bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
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
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), bidderDenom),
            message = "All bidder denom [$bidderDenom] should be returned to the bidder after cancelling the bid",
        )
    }

    @Test
    fun testUpdateAsk() {
        val markerDenom = "updateasksingletx"
        val shareCount = 100L
        val shareSaleAmount = 50.toBigInteger()
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
        val quoteDenom = "updateasksingletxsharesalequote"
        val askUuid = UUID.randomUUID()
        assertSucceeds("Creating the marker share sale ask should succeed") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = MarkerShareSaleAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        sharesToSell = shareSaleAmount,
                        quotePerShare = newCoins(100, quoteDenom),
                        shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        val response = assertSucceeds("Updating the marker share sale ask should succeed") {
            bilateralClient.updateAsk(
                updateAsk = UpdateAsk(
                    ask = MarkerShareSaleAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        sharesToSell = shareSaleAmount + BigInteger.TEN,
                        quotePerShare = newCoins(5000, quoteDenom),
                        shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        assertEquals(
            expected = askUuid.toString(),
            actual = response.askId,
            message = "The response should include the correct ask id",
        )
        assertEquals(
            expected = bilateralClient.getAsk(response.askId),
            actual = response.updatedAskOrder,
            message = "The response should include the updated ask order",
        )
        bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = newCoins(5000, quoteDenom),
            actual = response.updatedAskOrder.testGetMarkerShareSale().quotePerShare,
            message = "Expected the ask's quote per share to be properly updated",
        )
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
    }

    @Test
    fun testUpdateBid() {
        val markerDenom = "updatebidsingletx"
        val shareCount = 100L
        val shareSaleAmount = 50.toBigInteger()
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
        val quoteDenom = "updateasksingletxsharesalequote"
        val quoteDenom2 = "updateasksingletxsharesalequote2"
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
        assertSucceeds("Creating the marker share sale ask should succeed") {
            bilateralClient.createAsk(
                createAsk = CreateAsk(
                    ask = MarkerShareSaleAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        sharesToSell = shareSaleAmount,
                        quotePerShare = newCoins(100, quoteDenom),
                        shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.askerAccount,
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        assertSucceeds("Creating a bid should succeed") {
            bilateralClient.createBid(
                createBid = CreateBid(
                    bid = MarkerShareSaleBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        shareCount = shareSaleAmount,
                        quote = newCoins(50, quoteDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = 950,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The correct quote amount should be taken from the bidder's account",
        )
        val response = assertSucceeds("Updating a bid should succeed") {
            bilateralClient.updateBid(
                updateBid = UpdateBid(
                    bid = MarkerShareSaleBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        shareCount = shareSaleAmount,
                        quote = newCoins(999, quoteDenom2),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
                signer = BilateralAccounts.bidderAccount,
            )
        }
        assertEquals(
            expected = bidUuid.toString(),
            actual = response.bidId,
            message = "The response should include the correct bid id",
        )
        assertEquals(
            expected = bilateralClient.getBid(response.bidId),
            actual = response.updatedBidOrder,
            message = "The response should include the updated bid order",
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = newCoins(999, quoteDenom2),
            actual = response.updatedBidOrder.testGetMarkerShareSale().quote,
            message = "Expected the bid's quote to be properly updated",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom),
            message = "The bidder's quote should be fully refunded from the original bid",
        )
        assertEquals(
            expected = 1,
            actual = pbClient.getBalance(BilateralAccounts.bidderAccount.address(), quoteDenom2),
            message = "The bidder's quote2 should be debited down by the appropriate amount",
        )
        bilateralClient.cancelAsk(askUuid.toString(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.cancelBid(bidUuid.toString(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }
}
