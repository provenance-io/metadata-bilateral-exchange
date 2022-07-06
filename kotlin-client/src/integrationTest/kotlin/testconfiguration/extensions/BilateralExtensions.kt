package testconfiguration.extensions

import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.client.BilateralContractClient
import io.provenance.bilateral.execute.UpdateSettings
import testconfiguration.accounts.BilateralAccounts
import kotlin.test.assertEquals

fun BilateralContractClient.setFees(askFee: List<Coin>? = null, bidFee: List<Coin>? = null) {
    updateSettings(UpdateSettings.new(askFee = askFee, bidFee = bidFee), BilateralAccounts.adminAccount)
    val contractInfo = this.getContractInfo()
    assertEquals(
        expected = askFee,
        actual = contractInfo.askFee,
        message = "Expected ask fee to be properly set after updating it",
    )
    assertEquals(
        expected = bidFee,
        actual = contractInfo.bidFee,
        message = "Expected the bid fee to be properly set after updating it",
    )
}

fun BilateralContractClient.clearFees() = setFees()
