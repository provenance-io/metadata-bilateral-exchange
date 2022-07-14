package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.Ask.CoinTradeAsk
import io.provenance.bilateral.execute.Bid.CoinTradeBid
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
        val createAsk = CreateAsk(
            ask = CoinTradeAsk(
                id = coinTradeAskUuid.toString(),
                quote = newCoins(100, "nhash"),
                base = newCoins(150, "nhash"),
            ),
        )
        bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(createAsk.ask.mapToId())
        bilateralClient.getAskOrNull(createAsk.ask.mapToId()).assertNotNull("Ask should exist when fetched by nullable request")
        assertSucceeds("Expected the ask to be available by collateral id") { bilateralClient.getAskByCollateralId(coinTradeAskUuid.toString()) }
        bilateralClient.getAskByCollateralIdOrNull(coinTradeAskUuid.toString()).assertNotNull("ask should not be null when fetching by collateral id")
        bilateralClient.cancelAsk(createAsk.ask.mapToId(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(createAsk.ask.mapToId())
    }

    @Test
    fun testGetBidFunctions() {
        assertFails("A missing id should cause an exception for getBid") { bilateralClient.getBid("some fake bid") }
        bilateralClient.getBidOrNull("some id or whatever").assertNull("The OrNull variant of getBid should return null for a missing id")
        val coinTradeBidUuid = UUID.randomUUID()
        val createBid = CreateBid(
            bid = CoinTradeBid(
                id = coinTradeBidUuid.toString(),
                quote = newCoins(100, "nhash"),
                base = newCoins(150, "nhash"),
            ),
        )
        bilateralClient.createBid(createBid, BilateralAccounts.bidderAccount)
        bilateralClient.assertBidExists(createBid.bid.mapToId())
        bilateralClient.getBidOrNull(createBid.bid.mapToId()).assertNotNull("Bid should exist when fetched by nullable request")
        bilateralClient.cancelBid(createBid.bid.mapToId(), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(createBid.bid.mapToId())
    }
}
