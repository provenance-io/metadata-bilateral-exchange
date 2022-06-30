package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.CreateAsk
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertNotNull
import testconfiguration.functions.assertNull
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.util.UUID
import kotlin.test.assertFails

class QueryIntTest : ContractIntTest() {
    @Test
    fun testGetAskByCollateralId() {
        assertFails("A missing id should cause an exception") { bilateralClient.getAskByCollateralId("some id whatever") }
        bilateralClient.getAskByCollateralIdOrNull("fakeid").assertNull("The OrNull variant should return null for a missing id")
        val coinTradeAskUuid = UUID.randomUUID()
        testAndCleanupAsk(
            ask = CreateAsk.newCoinTrade(
                id = coinTradeAskUuid.toString(),
                quote = newCoins(100, "nhash"),
                base = newCoins(150, "nhash"),
                descriptor = null,
            ),
            expectedId = coinTradeAskUuid.toString(),
        )
    }

    private fun testAndCleanupAsk(ask: CreateAsk, expectedId: String) {
        bilateralClient.createAsk(ask, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(ask.getId())
        assertSucceeds("Expected the ask to be available by collateral id") { bilateralClient.getAskByCollateralId(expectedId) }
        bilateralClient.getAskByCollateralIdOrNull(expectedId).assertNotNull("ask should not be null when fetching by collateral id")
        bilateralClient.cancelAsk(ask.getId(), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(ask.getId())
    }
}
