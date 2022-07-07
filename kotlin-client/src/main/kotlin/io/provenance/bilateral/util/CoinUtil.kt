package io.provenance.bilateral.util

import cosmos.base.v1beta1.CoinOuterClass.Coin
import java.math.BigDecimal

internal object CoinUtil {
    fun combineFunds(first: List<Coin>, second: List<Coin>): List<Coin> =
        first.groupBy { it.denom }
            .toMutableMap()
            // Append each coin from the second list into the map of the first list's coins, generating new entries
            // when they don't exist
            .also { firstCoinMap ->
                second.forEach { secondCoin ->
                    firstCoinMap[secondCoin.denom]
                        ?.also { existingCoinList -> firstCoinMap[secondCoin.denom] = existingCoinList.plus(secondCoin) }
                        ?: run { firstCoinMap[secondCoin.denom] = listOf(secondCoin) }
                }
            }
            .entries
            // Collapse each map entry's coins from both collections into the aggregate of their amounts
            .map { (denom, coins) ->
                Coin.newBuilder()
                    .setDenom(denom)
                    .setAmount(
                        coins
                            .fold(BigDecimal.ZERO) { acc, coin ->
                                // Convert coin values to BigDecimal and only hold on to values that are above zero at
                                // least, ensuring real amounts aren't reduced by invalid values
                                acc + (coin.amount.toBigDecimalOrNull()?.takeIf { it >= BigDecimal.ZERO } ?: BigDecimal.ZERO)
                            }
                            .toPlainString()
                    )
                    .build()
            }
            // Ignore coins that have zero denom and/or amounts that cannot be represented as digits
            .filter { coin ->
                coin.denom.isNotBlank() &&
                    coin.amount.toBigDecimalOrNull()?.let { amount -> amount.setScale(0) > BigDecimal.ZERO } == true
            }
}
