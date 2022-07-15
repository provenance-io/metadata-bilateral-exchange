package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.MarkerShareSaleAsk
import io.provenance.bilateral.execute.Ask.MarkerTradeAsk
import io.provenance.bilateral.execute.Bid.CoinTradeBid
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
import testconfiguration.ContractIntTest
import testconfiguration.extensions.getBalance
import testconfiguration.extensions.getMarkerAccount
import testconfiguration.extensions.testGetCoinTrade
import testconfiguration.extensions.testGetMarkerShareSale
import testconfiguration.extensions.testGetMarkerTrade
import testconfiguration.functions.assertSingle
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.createMarker
import testconfiguration.functions.giveTestDenom
import testconfiguration.functions.grantMarkerAccess
import testconfiguration.functions.newCoin
import testconfiguration.functions.newCoins
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
            ownerAccount = asker,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val bidderDenom = "singletxbid"
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(50, bidderDenom),
            receiverAddress = bidder.address(),
        )
        val askUuid = UUID.randomUUID()
        createAsk(
            createAsk = CreateAsk(
                ask = MarkerShareSaleAsk(
                    id = askUuid.toString(),
                    markerDenom = markerDenom,
                    sharesToSell = shareSaleAmount,
                    quotePerShare = newCoins(1, bidderDenom),
                    shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                ),
                descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
            )
        )
        assertEquals(
            expected = contractInfo.contractAddress,
            actual = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single account should have marker permissions once the ask is created").address,
            message = "The contract should control the marker account after creating a marker share sale",
        )
        assertFails("An ask the for same marker should not be allowed once a marker share sale is created") {
            createAsk(
                createAsk = CreateAsk(
                    ask = MarkerShareSaleAsk(
                        id = UUID.randomUUID().toString(),
                        markerDenom = markerDenom,
                        sharesToSell = shareSaleAmount,
                        quotePerShare = newCoins(1, bidderDenom),
                        shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                    ),
                ),
            )
        }
        val bidUuid = UUID.randomUUID()
        createBid(
            createBid = CreateBid(
                bid = MarkerShareSaleBid(
                    id = bidUuid.toString(),
                    markerDenom = markerDenom,
                    shareCount = shareSaleAmount,
                    quote = newCoins(50, bidderDenom),
                ),
                descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
            ),
        )
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(bidder.address(), bidderDenom),
            message = "The bidder's denom [$bidderDenom] should be held in escrow when the bid is created",
        )
        val executeMatchResponse = executeMatch(ExecuteMatch(askUuid.toString(), bidUuid.toString()))
        assertTrue(
            actual = executeMatchResponse.askDeleted,
            message = "The response should indicate that the ask was deleted",
        )
        assertTrue(
            actual = executeMatchResponse.bidDeleted,
            message = "The response should indicate that the bid was deleted",
        )
        assertEquals(
            expected = 50L,
            actual = pbClient.getBalance(asker.address(), bidderDenom),
            message = "The correct amount of [$bidderDenom] denom should be transferred to the asker",
        )
        val grant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single access should remain on the marker after executing the trade")
        assertEquals(
            expected = asker.address(),
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
            actual = pbClient.getBalance(bidder.address(), markerDenom),
            message = "The correct amount of marker denom [$markerDenom] should be transferred to the bidder",
        )
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(bidder.address(), bidderDenom),
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
            ownerAccount = asker,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val bidderDenom = "multipletxbid"
        var expectedBidderDenomHoldings = 75L
        giveTestDenom(
            pbClient = pbClient,
            initialHoldings = newCoin(expectedBidderDenomHoldings, bidderDenom),
            receiverAddress = bidder.address(),
        )
        val askUuid = UUID.randomUUID()
        createAsk(
            createAsk = CreateAsk(
                ask = MarkerShareSaleAsk(
                    id = askUuid.toString(),
                    markerDenom = markerDenom,
                    sharesToSell = sharesToSell,
                    quotePerShare = newCoins(1, bidderDenom),
                    shareSaleType = ShareSaleType.MULTIPLE_TRANSACTIONS,
                ),
                descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
            )
        )
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
            createBid(
                createBid = CreateBid(
                    bid = MarkerShareSaleBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        shareCount = sharePurchaseCount,
                        // Pay 1 bidderDenom per share
                        quote = newCoins(sharePurchaseCount.toLong(), bidderDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
            )
            expectedBidderDenomHoldings -= sharePurchaseCount.toLong()
            assertEquals(
                expected = expectedBidderDenomHoldings,
                actual = pbClient.getBalance(bidder.address(), bidderDenom),
                message = "Expected the proper amount of denom [$bidderDenom] to be taken from the bidder as the quote",
            )
            val executeMatchResponse = executeMatch(ExecuteMatch(askUuid.toString(), bidUuid.toString()))
            assertTrue(
                actual = executeMatchResponse.bidDeleted,
                message = "The execute match response should indicate that the bid was deleted",
            )
            expectedBidderMarkerHoldings += sharePurchaseCount.toLong()
            expectedAskerDenomHoldings += sharePurchaseCount.toLong()
            assertEquals(
                expected = expectedBidderMarkerHoldings,
                actual = pbClient.getBalance(bidder.address(), markerDenom),
                message = "Expected the bidder to hold the correct amount of the marker denom [$markerDenom]"
            )
            assertEquals(
                expected = expectedAskerDenomHoldings,
                actual = pbClient.getBalance(asker.address(), bidderDenom),
                message = "Expected the asker to have received the correct amount of test denom [$bidderDenom]",
            )
            if (counter != maxIteration) {
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
        val grant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("A single access grant should exist on the marker")
        assertEquals(
            expected = asker.address(),
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
            ownerAccount = asker,
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
            createAsk(createAsk)
        }
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN),
        )
        assertFails("An ask cannot bbe created when the contract only has admin permissions. It also needs withdraw") {
            createAsk(createAsk)
        }
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val createResponse = assertSucceeds("An ask should be allowed when the marker has the correct permission structure") {
            createAsk(createAsk)
        }
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
            createAsk(
                createAsk = CreateAsk(
                    ask = MarkerShareSaleAsk(
                        id = UUID.randomUUID().toString(),
                        markerDenom = markerDenom,
                        sharesToSell = 100.toBigInteger(),
                        quotePerShare = newCoins(1, "nhash"),
                        shareSaleType = ShareSaleType.SINGLE_TRANSACTION,
                    ),
                ),
            )
        }
        val cancelResponse = cancelAsk(askUuid.toString())
        assertEquals(
            expected = createResponse.askOrder,
            actual = cancelResponse.cancelledAskOrder,
            message = "The cancelled ask order should be included in the response",
        )
        val afterCancelGrant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single permission should exist on the marker after cancelling the ask")
        assertEquals(
            expected = asker.address(),
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
            receiverAddress = bidder.address(),
            sendAmount = 100L,
        )
        val bidUuid = UUID.randomUUID()
        val createResponse = createBid(
            createBid = CreateBid(
                bid = MarkerShareSaleBid(
                    id = bidUuid.toString(),
                    markerDenom = bidderDenom,
                    shareCount = 100.toBigInteger(),
                    quote = newCoins(100, bidderDenom),
                ),
            ),
        )
        assertEquals(
            expected = 0L,
            actual = pbClient.getBalance(bidder.address(), bidderDenom),
            message = "All bidder denom [$bidderDenom] should be held in contract escrow after creating the bid",
        )
        val cancelResponse = cancelBid(bidUuid.toString())
        assertEquals(
            expected = createResponse.bidOrder,
            actual = cancelResponse.cancelledBidOrder,
            message = "Expected the cancelled bid order to be included in the response",
        )
        assertEquals(
            expected = 100L,
            actual = pbClient.getBalance(bidder.address(), bidderDenom),
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
            ownerAccount = asker,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val quoteDenom = "updateasksingletxsharesalequote"
        val askUuid = UUID.randomUUID()
        assertSucceeds("Creating the marker share sale ask should succeed") {
            createAsk(
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
            )
        }
        val updateResponse = assertSucceeds("Updating the marker share sale ask should succeed") {
            updateAsk(
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
            )
        }
        assertEquals(
            expected = newCoins(5000, quoteDenom),
            actual = updateResponse.updatedAskOrder.testGetMarkerShareSale().quotePerShare,
            message = "Expected the ask's quote per share to be properly updated",
        )
        val cancelResponse = cancelAsk(askUuid.toString())
        assertEquals(
            expected = updateResponse.updatedAskOrder,
            actual = cancelResponse.cancelledAskOrder,
            message = "The cancelled ask order should be returned in the response",
        )
        val afterCancelGrant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single permission should exist on the marker after cancelling the ask")
        assertEquals(
            expected = asker.address(),
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
    fun testUpdateAskToMarkerTrade() {
        val markerDenom = "updateasktomarkertradesingletx"
        val shareCount = 100L
        val shareSaleAmount = 50.toBigInteger()
        val markerPermissions = listOf(Access.ACCESS_ADMIN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = asker,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val quoteDenom = "updateasktomarkertradesingletxsharesalequote"
        val askUuid = UUID.randomUUID()
        assertSucceeds("Creating the marker share sale ask should succeed") {
            createAsk(
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
            )
        }
        val updateResponse = assertSucceeds("Updating the marker share sale ask should succeed") {
            updateAsk(
                updateAsk = UpdateAsk(
                    ask = MarkerTradeAsk(
                        id = askUuid.toString(),
                        markerDenom = markerDenom,
                        quotePerShare = newCoins(5000, quoteDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
            )
        }
        val tradeCollateral = updateResponse.updatedAskOrder.testGetMarkerTrade()
        assertEquals(
            expected = newCoins(5000, quoteDenom),
            actual = tradeCollateral.quotePerShare,
            message = "Expected the ask's quote per share to be properly updated",
        )
        assertEquals(
            expected = shareCount.toBigInteger(),
            actual = tradeCollateral.shareCount,
            message = "The marker's total share count should be stored in the ask order",
        )
        val cancelResponse = cancelAsk(askUuid.toString())
        assertEquals(
            expected = updateResponse.updatedAskOrder,
            actual = cancelResponse.cancelledAskOrder,
            message = "The cancelled ask order should be returned in the response",
        )
        val afterCancelGrant = pbClient.getMarkerAccount(markerDenom).accessControlList.assertSingle("Only a single permission should exist on the marker after cancelling the ask")
        assertEquals(
            expected = asker.address(),
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
    fun testUpdateBidToSameType() {
        val markerDenom = "updatebidsingletx"
        val shareCount = 100L
        val shareSaleAmount = 50.toBigInteger()
        val markerPermissions = listOf(Access.ACCESS_ADMIN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = asker,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val quoteDenom = "updatebidsingletxsharesalequote"
        val quoteDenom2 = "updatebidsingletxsharesalequote2"
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
        assertSucceeds("Creating the marker share sale ask should succeed") {
            createAsk(
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
            )
        }
        val bidUuid = UUID.randomUUID()
        assertSucceeds("Creating a bid should succeed") {
            createBid(
                createBid = CreateBid(
                    bid = MarkerShareSaleBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        shareCount = shareSaleAmount,
                        quote = newCoins(50, quoteDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
            )
        }
        assertEquals(
            expected = 950,
            actual = pbClient.getBalance(bidder.address(), quoteDenom),
            message = "The correct quote amount should be taken from the bidder's account",
        )
        val response = assertSucceeds("Updating a bid should succeed") {
            updateBid(
                updateBid = UpdateBid(
                    bid = MarkerShareSaleBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        shareCount = shareSaleAmount,
                        quote = newCoins(999, quoteDenom2),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
            )
        }
        assertEquals(
            expected = newCoins(999, quoteDenom2),
            actual = response.updatedBidOrder.testGetMarkerShareSale().quote,
            message = "Expected the bid's quote to be properly updated",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(bidder.address(), quoteDenom),
            message = "The bidder's quote should be fully refunded from the original bid",
        )
        assertEquals(
            expected = 1,
            actual = pbClient.getBalance(bidder.address(), quoteDenom2),
            message = "The bidder's quote2 should be debited down by the appropriate amount",
        )
    }

    @Test
    fun testUpdateBidToNewType() {
        val markerDenom = "updatebidnewtypesingletx"
        val shareCount = 100L
        val shareSaleAmount = 50.toBigInteger()
        val markerPermissions = listOf(Access.ACCESS_ADMIN)
        createMarker(
            pbClient = pbClient,
            ownerAccount = asker,
            denomName = markerDenom,
            supply = shareCount,
            permissions = markerPermissions,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = asker,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val quoteDenom = "updatebidnewtypesingletxsharesalequote"
        val quoteDenom2 = "updatebidnewtypesingletxsharesalequote2"
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
        assertSucceeds("Creating the marker share sale ask should succeed") {
            createAsk(
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
            )
        }
        val bidUuid = UUID.randomUUID()
        assertSucceeds("Creating a bid should succeed") {
            createBid(
                createBid = CreateBid(
                    bid = MarkerShareSaleBid(
                        id = bidUuid.toString(),
                        markerDenom = markerDenom,
                        shareCount = shareSaleAmount,
                        quote = newCoins(50, quoteDenom),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
            )
        }
        assertEquals(
            expected = 950,
            actual = pbClient.getBalance(bidder.address(), quoteDenom),
            message = "The correct quote amount should be taken from the bidder's account",
        )
        val response = assertSucceeds("Updating a bid should succeed") {
            updateBid(
                updateBid = UpdateBid(
                    bid = CoinTradeBid(
                        id = bidUuid.toString(),
                        quote = newCoins(999, quoteDenom2),
                        base = newCoins(50, "somebase"),
                    ),
                    descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
                ),
            )
        }
        val collateral = response.updatedBidOrder.testGetCoinTrade()
        assertEquals(
            expected = newCoins(999, quoteDenom2),
            actual = collateral.quote,
            message = "Expected the bid's quote to be properly updated",
        )
        assertEquals(
            expected = newCoins(50, "somebase"),
            actual = collateral.base,
            message = "Expected the bid's base to be properly updated",
        )
        assertEquals(
            expected = 1000,
            actual = pbClient.getBalance(bidder.address(), quoteDenom),
            message = "The bidder's quote should be fully refunded from the original bid",
        )
        assertEquals(
            expected = 1,
            actual = pbClient.getBalance(bidder.address(), quoteDenom2),
            message = "The bidder's quote2 should be debited down by the appropriate amount",
        )
    }
}
