package io.provenance.bilateral.contract

import io.provenance.bilateral.client.BroadcastOptions
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.RequestDescriptor
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

class MarkerTradeIntTest : ContractIntTest() {
    @Test
    fun testSimpleMarkerTrade() {
        val markerDenom = "testdenom"
        createMarker(
            pbClient = pbClient,
            ownerAccount = BilateralAccounts.askerAccount,
            denomName = markerDenom,
            supply = 10,
        )
        grantMarkerAccess(
            pbClient = pbClient,
            markerAdminAccount = BilateralAccounts.askerAccount,
            markerDenom = markerDenom,
            grantAddress = contractInfo.contractAddress,
        )
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newMarkerTrade(
            id = askUuid.toString(),
            denom = markerDenom,
            quotePerShare = newCoins(50, "nhash"),
            descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
        )
        bilateralClient.createAsk(createAsk = createAsk, signer = BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newMarkerTrade(
            id = bidUuid.toString(),
            denom = markerDenom,
            quote = newCoins(500, "nhash"),
            descriptor = RequestDescriptor(description = "Example description", effectiveTime = OffsetDateTime.now()),
        )
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        bilateralClient.executeMatch(
            executeMatch = ExecuteMatch.new(askUuid.toString(), bidUuid.toString()),
            signer = BilateralAccounts.adminAccount,
        )
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }
}
