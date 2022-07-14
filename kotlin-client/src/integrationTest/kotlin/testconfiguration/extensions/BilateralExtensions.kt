package testconfiguration.extensions

import cosmos.base.v1beta1.CoinOuterClass.Coin
import io.provenance.bilateral.client.BilateralContractClient
import io.provenance.bilateral.execute.UpdateSettings
import io.provenance.bilateral.models.AskCollateral
import io.provenance.bilateral.models.AskOrder
import io.provenance.bilateral.models.BidCollateral
import io.provenance.bilateral.models.BidOrder
import testconfiguration.accounts.BilateralAccounts
import kotlin.test.assertEquals
import kotlin.test.fail

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

fun AskOrder.testGetCoinTrade(): AskCollateral.CoinTrade = testGetAskCollateral()

fun AskOrder.testGetMarkerTrade(): AskCollateral.MarkerTrade = testGetAskCollateral()

fun AskOrder.testGetMarkerShareSale(): AskCollateral.MarkerShareSale = testGetAskCollateral()

fun AskOrder.testGetScopeTrade(): AskCollateral.ScopeTrade = testGetAskCollateral()

fun BidOrder.testGetCoinTrade(): BidCollateral.CoinTrade = testGetBidCollateral()

fun BidOrder.testGetMarkerTrade(): BidCollateral.MarkerTrade = testGetBidCollateral()

fun BidOrder.testGetMarkerShareSale(): BidCollateral.MarkerShareSale = testGetBidCollateral()

fun BidOrder.testGetScopeTrade(): BidCollateral.ScopeTrade = testGetBidCollateral()

private inline fun <reified T : AskCollateral> AskOrder.testGetAskCollateral(): T = collateral.let { c ->
    when (c) {
        is T -> c
        else -> fail("Expected ask collateral to be of type [${T::class.simpleName}], but was [${c::class.simpleName}]")
    }
}

private inline fun <reified T : BidCollateral> BidOrder.testGetBidCollateral(): T = collateral.let { c ->
    when (c) {
        is T -> c
        else -> fail("Expected bid collateral to be of type [${T::class.simpleName}], but was [${c::class.simpleName}]")
    }
}
