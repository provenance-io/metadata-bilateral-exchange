package testconfiguration.functions

import cosmos.base.v1beta1.CoinOuterClass.Coin

fun newCoin(amount: Long, denom: String): Coin = Coin.newBuilder().setAmount(amount.toString()).setDenom(denom).build()

fun newCoins(amount: Long, denom: String): List<Coin> = listOf(newCoin(amount, denom))
