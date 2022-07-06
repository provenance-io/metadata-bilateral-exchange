package io.provenance.bilateral.util

import cosmos.base.v1beta1.CoinOuterClass.Coin
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

class CoinUtilTest {
    @Test
    fun testCombineFundsWithEmptyLists() {
        assertEquals(
            expected = emptyList(),
            actual = CoinUtil.combineFunds(emptyList(), emptyList()),
            message = "Combining two empty funds lists should result in an empty list",
        )
    }

    @Test
    fun testCombineFundsWithOnlyOnePopulatedList() {
        val coins = newCoins("100", "test")
        assertEquals(
            expected = coins,
            actual = CoinUtil.combineFunds(coins, emptyList()),
            message = "Combining one populated list with an empty list in the second position should result in the initial value",
        )
        assertEquals(
            expected = coins,
            actual = CoinUtil.combineFunds(emptyList(), coins),
            message = "Combining one populated list with an empty list in the first position should result in the initial value",
        )
    }

    @Test
    fun testInvalidCoinAmountsAreRemoved() {
        val invalidCoins1 = listOf(
            newCoin(amount = "not a number", denom = "nhash"),
            newCoin(amount = "100", denom = ""),
            newCoin(amount = "55", denom = "    "),
        )
        val invalidCoins2 = newCoins(amount = "-25", denom = "fakehash")
        assertEquals(
            expected = emptyList(),
            actual = CoinUtil.combineFunds(invalidCoins1, invalidCoins2),
            message = "All types of invalid coins should be removed from the list, producing an empty result",
        )
    }

    @Test
    fun testSingleCoinCombine() {
        val first = newCoins(amount = "100", denom = "nhash")
        val second = newCoins(amount = "200", denom = "nhash")
        assertEquals(
            expected = newCoins(amount = "300", denom = "nhash"),
            actual = CoinUtil.combineFunds(first, second),
            message = "Both nhash amounts should be combined to produce 300nhash",
        )
    }

    @Test
    fun testMultipleCoinCombine() {
        val first = listOf(
            newCoin(amount = "10", denom = "a"),
            newCoin(amount = "10", denom = "b"),
        )
        val second = listOf(
            newCoin(amount = "90", denom = "a"),
            newCoin(amount = "190", denom = "b"),
        )
        assertEquals(
            expected = listOf(
                newCoin(amount = "100", denom = "a"),
                newCoin(amount = "200", denom = "b"),
            ).sorted(),
            actual = CoinUtil.combineFunds(first, second).sorted(),
            message = "Expected result to accurately combine coin values",
        )
    }

    @Test
    fun testDifferentCoinTypeCombination() {
        val first = newCoins(amount = "100", denom = "a")
        val second = newCoins(amount = "100", denom = "b")
        assertEquals(
            expected = listOf(first, second).flatten().sorted(),
            actual = CoinUtil.combineFunds(first, second).sorted(),
            message = "Expected the function to properly combine different coin types into a single list",
        )
    }

    @Test
    fun testCombinationsAndAdditionsFromBothLists() {
        val first = listOf(
            newCoin(amount = "10", denom = "a"),
            newCoin(amount = "50", denom = "b"),
            newCoin(amount = "1", denom = "c"),
        )
        val second = listOf(
            newCoin(amount = "2", denom = "c"),
            newCoin(amount = "55", denom = "d"),
            newCoin(amount = "44", denom = "e"),
        )
        assertEquals(
            expected = listOf(
                newCoin(amount = "10", denom = "a"),
                newCoin(amount = "50", denom = "b"),
                newCoin(amount = "3", denom = "c"),
                newCoin(amount = "55", denom = "d"),
                newCoin(amount = "44", denom = "e"),
            ).sorted(),
            actual = CoinUtil.combineFunds(first, second).sorted(),
            message = "Expected the result to properly combine denom \"c\" and just add the other values",
        )
    }

    private fun newCoin(amount: String, denom: String): Coin = Coin.newBuilder().setAmount(amount).setDenom(denom).build()

    private fun newCoins(amount: String, denom: String): List<Coin> = newCoin(amount, denom).let(::listOf)

    private fun Collection<Coin>.sorted(): List<Coin> = this.sortedBy { it.denom }.sortedBy { it.amount }
}
