package io.provenance.bilateral.client

sealed interface BilateralContractClientLogger {
    fun info(message: String)
    fun error(message: String, e: Exception?)

    object Off : BilateralContractClientLogger {
        override fun info(message: String) {}
        override fun error(message: String, e: Exception?) {}
    }

    object Println : BilateralContractClientLogger {
        override fun info(message: String) {
            println(message)
        }

        override fun error(message: String, e: Exception?) {
            System.err.println("$message${e?.let { exception -> ": ${exception.message}"} ?: ""}")
        }
    }

    class Custom(
        private val infoLogger: (message: String) -> Unit,
        private val errorLogger: (message: String, e: Exception?) -> Unit,
    ) : BilateralContractClientLogger {
        override fun info(message: String) = infoLogger.invoke(message)
        override fun error(message: String, e: Exception?) = errorLogger.invoke(message, e)
    }
}
