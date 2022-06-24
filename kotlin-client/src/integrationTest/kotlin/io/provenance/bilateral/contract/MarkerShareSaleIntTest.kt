package io.provenance.bilateral.contract

import io.provenance.bilateral.client.BroadcastOptions
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.bilateral.models.ShareSaleType
import io.provenance.marker.v1.Access
import mu.KLogging
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.createMarker
import testconfiguration.functions.grantMarkerAccess
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.time.OffsetDateTime
import java.util.UUID

class MarkerShareSaleIntTest : ContractIntTest() {
    private companion object : KLogging()

    @Test
    fun testSingleTxShareSale() {
        val markerDenom = "singletx"
        val shareCount = 100L
        val shareSaleAmount = 50L
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = shareCount
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom =  markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newMarkerShareSale(
            id = askUuid.toString(),
            denom = markerDenom,
            quotePerShare = newCoins(50, "nhash"),
            shareSaleType = ShareSaleType.single(shareSaleAmount.toString()),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
        )
        bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newMarkerShareSale(
            id = bidUuid.toString(),
            denom = markerDenom,
            shareCount = shareSaleAmount.toString(),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
        )
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
            options = BroadcastOptions(newCoins(50 * shareSaleAmount, "nhash")),
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        val executeMatch = ExecuteMatch.new(askUuid.toString(), bidUuid.toString())
        bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }

    @Test
    fun testMultipleTxShareSale() {
        val markerDenom = "multipletx"
        val shareCount = 100L
        val sharePurchaseCount = 25L
        val shareCutoff = 25L
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = shareCount
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
            permissions = listOf(Access.ACCESS_ADMIN, Access.ACCESS_WITHDRAW),
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newMarkerShareSale(
            id = askUuid.toString(),
            denom = markerDenom,
            quotePerShare = newCoins(1000, "nhash"),
            shareSaleType = ShareSaleType.multiple(removeSaleShareThreshold = shareCutoff.toString()),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
        )
        bilateralClient.createAsk(
            createAsk = createAsk,
            signer = BilateralAccounts.askerAccount,
        )
        bilateralClient.assertAskExists(askUuid.toString())
        val maxIteration = (shareCount - shareCutoff) / sharePurchaseCount - 1
        for (counter in 0..maxIteration) {
            val bidUuid = UUID.randomUUID()
            val createBid = CreateBid.newMarkerShareSale(
                id = bidUuid.toString(),
                denom = markerDenom,
                shareCount = sharePurchaseCount.toString(),
                descriptor = RequestDescriptor("Example description", OffsetDateTime.now())
            )
            bilateralClient.createBid(
                createBid = createBid,
                signer = BilateralAccounts.bidderAccount,
                // Pay 1000nhash per share purchased
                options = BroadcastOptions(funds = newCoins(1000 * sharePurchaseCount, "nhash")),
            )
            bilateralClient.assertBidExists(bidUuid.toString())
            val executeMatch = ExecuteMatch.new(askUuid.toString(), bidUuid.toString())
            bilateralClient.executeMatch(executeMatch = executeMatch, signer = BilateralAccounts.adminAccount)
            bilateralClient.assertBidIsDeleted(bidUuid.toString())
            if (counter == maxIteration) {
                bilateralClient.assertAskIsDeleted(askUuid.toString(), "Ask should be deleted because the share sale threshold has been reached")
            } else {
                bilateralClient.assertAskExists(askUuid.toString(), "Ask should still exist because the share threshold has not yet been reached")
            }
        }
    }
}
