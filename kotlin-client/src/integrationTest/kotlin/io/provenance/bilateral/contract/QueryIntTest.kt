package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertNotNull
import testconfiguration.functions.assertNull
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.util.UUID
import kotlin.test.assertFails

class QueryIntTest : ContractIntTest() {
    @Test
    fun testGetAskFunctions() {
        assertFails("A missing id should cause an exception for getAsk") { bilateralClient.getAsk("some fake ask") }
        assertFails("A missing id should cause an exception for getAskByCollateralId") { bilateralClient.getAskByCollateralId("some id whatever") }
        bilateralClient.getAskOrNull("some id or something").assertNull("The OrNull variant of getAsk should return null for a missing id")
        bilateralClient.getAskByCollateralIdOrNull("fakeid").assertNull("The OrNull variant of getAskByCollateralId should return null for a missing id")
        val coinTradeAskUuid = UUID.randomUUID()
        val ask = CreateAsk.newCoinTrade(
            id = coinTradeAskUuid.toString(),
            quote = newCoins(100, "nhash"),
            base = newCoins(150, "nhash"),
            descriptor = null,
        )
        bilateralClient.createAsk(ask, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(ask.getId())
        bilateralClient.getAskOrNull(ask.getId()).assertNotNull("Ask should exist when fetched by nullable request")
        assertSucceeds("Expected the ask to be available by collateral id") { bilateralClient.getAskByCollateralId(coinTradeAskUuid.toString()) }
        bilateralClient.getAskByCollateralIdOrNull(coinTradeAskUuid.toString()).assertNotNull("ask should not be null when fetching by collateral id")
        bilateralClient.cancelAsk(ask.getId(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(ask.getId())
    }

    @Test
    fun testGetBidFunctions() {
        assertFails("A missing id should cause an exception for getBid") { bilateralClient.getBid("some fake bid") }
        bilateralClient.getBidOrNull("some id or whatever").assertNull("The OrNull variant of getBid should return null for a missing id")
        val coinTradeBidUuid = UUID.randomUUID()
        val bid = CreateBid.newCoinTrade(
            id = coinTradeBidUuid.toString(),
            quote = newCoins(100, "nhash"),
            base = newCoins(150, "nhash"),
            descriptor = null,
        )
        bilateralClient.createBid(bid, BilateralAccounts.bidderAccount)
        bilateralClient.assertBidExists(bid.getId())
        bilateralClient.getBidOrNull(bid.getId()).assertNotNull("Bid should exist when fetched by nullable request")
        bilateralClient.cancelBid(bid.getId(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(bid.getId())
    }
}
