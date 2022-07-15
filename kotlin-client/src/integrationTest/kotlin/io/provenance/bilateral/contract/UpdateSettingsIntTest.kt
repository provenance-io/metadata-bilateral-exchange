package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.UpdateSettings
import org.junit.jupiter.api.Test
import testconfiguration.ContractIntTest
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.newCoins
import kotlin.test.assertEquals
import kotlin.test.assertFails

class UpdateSettingsIntTest : ContractIntTest() {
    @Test
    fun testUpdateSettingAll() {
        val firstUpdateSettings = UpdateSettings.new(
            newAdminAddress = BilateralAccounts.askerAccount.address(),
            askFee = newCoins(100, "askcoin"),
            bidFee = newCoins(100, "bidcoin"),
        )
        assertFails("An update not originating from the admin should be rejected") {
            bilateralClient.updateSettings(firstUpdateSettings, BilateralAccounts.askerAccount)
        }
        val firstResponse = assertSucceeds("A valid update sent from the admin should succeed") {
            bilateralClient.updateSettings(firstUpdateSettings, BilateralAccounts.adminAccount)
        }
        assertEquals(
            expected = BilateralAccounts.askerAccount.address(),
            actual = firstResponse.newAdminAddress,
            message = "The new admin address should be in the response",
        )
        assertEquals(
            expected = "100askcoin",
            actual = firstResponse.newAskFee,
            message = "The new ask fee should be in the response",
        )
        assertEquals(
            expected = "100bidcoin",
            actual = firstResponse.newBidFee,
            message = "The new bid fee should be in the response",
        )
        assertContractInfoValuesWereChanged(firstUpdateSettings)
        // Put everything back to normal
        val secondUpdateSettings = UpdateSettings.new(
            newAdminAddress = BilateralAccounts.adminAccount.address(),
            askFee = null,
            bidFee = null,
        )
        assertFails("An update not originating from the asker should be rejected") {
            bilateralClient.updateSettings(secondUpdateSettings, BilateralAccounts.adminAccount)
        }
        val secondResponse = assertSucceeds("A valid update sent from the asker should succeed") {
            bilateralClient.updateSettings(secondUpdateSettings, BilateralAccounts.askerAccount)
        }
        assertEquals(
            expected = BilateralAccounts.adminAccount.address(),
            actual = secondResponse.newAdminAddress,
            message = "The new new admin address should be in the response",
        )
        assertEquals(
            expected = "none",
            actual = secondResponse.newAskFee,
            message = "The new ask fee should be none, indicating that it was cleared",
        )
        assertEquals(
            expected = "none",
            actual = secondResponse.newBidFee,
            message = "The new bid fee should be none, indicating that it was cleared",
        )
        assertContractInfoValuesWereChanged(secondUpdateSettings)
    }

    private fun assertContractInfoValuesWereChanged(updateSettings: UpdateSettings) {
        val contractInfo = bilateralClient.getContractInfo()
        assertEquals(
            expected = updateSettings.update.newAdminAddress,
            actual = contractInfo.admin,
            message = "The admin address should be properly altered to match the request",
        )
        assertEquals(
            expected = updateSettings.update.askFee,
            actual = contractInfo.askFee,
            message = "The ask fee should be properly altered to match the request",
        )
        assertEquals(
            expected = updateSettings.update.bidFee,
            actual = contractInfo.bidFee,
            message = "The bid fee should be properly altered to match the request",
        )
    }
}
