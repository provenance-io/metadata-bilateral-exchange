package io.provenance.bilateral.contract

import io.provenance.bilateral.client.BroadcastOptions
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.AttributeRequirement
import io.provenance.bilateral.models.AttributeRequirementType
import io.provenance.bilateral.models.RequestDescriptor
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.time.OffsetDateTime
import java.util.UUID

class CoinTradeIntTest : ContractIntTest() {
    @Test
    fun testCoinTradeCompleteFlow() {
        // Simple trade of nhash for nhash
        val quote = newCoins(150, "nhash")
        val base = newCoins(200, "nhash")
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newCoinTrade(
            id = askUuid.toString(),
            quote = quote,
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(listOf("a.pb", "b.pb"), AttributeRequirementType.NONE),
            )
        )
        assertSucceeds("Ask should be created without error") {
            bilateralClient.createAsk(
                createAsk = createAsk,
                signer = BilateralAccounts.askerAccount,
                options = BroadcastOptions(funds = base),
            )
        }
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newCoinTrade(
            id = bidUuid.toString(),
            base = base,
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(listOf("c.pb"), AttributeRequirementType.NONE),
            ),
        )
        assertSucceeds("Bid should be created without error") {
            bilateralClient.createBid(
                createBid = createBid,
                signer = BilateralAccounts.bidderAccount,
                options = BroadcastOptions(funds = quote),
            )
        }
        bilateralClient.assertBidExists(bidUuid.toString())
        val executeMatch = ExecuteMatch.new(
            askId = askUuid.toString(),
            bidId = bidUuid.toString(),
        )
        assertSucceeds("Match shhould be executed without error") {
            bilateralClient.executeMatch(
                executeMatch = executeMatch,
                signer = BilateralAccounts.adminAccount,
            )
        }
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }
}
