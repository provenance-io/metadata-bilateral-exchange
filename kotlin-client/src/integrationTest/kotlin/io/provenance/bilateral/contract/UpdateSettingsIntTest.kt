package io.provenance.bilateral.contract

import io.provenance.bilateral.execute.UpdateSettings
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
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
        assertSucceeds("A valid update sent from the admin should succeed") {
            bilateralClient.updateSettings(firstUpdateSettings, BilateralAccounts.adminAccount)
        }
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
        assertSucceeds("A valid update sent from the asker should succeed") {
            bilateralClient.updateSettings(secondUpdateSettings, BilateralAccounts.askerAccount)
        }
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
