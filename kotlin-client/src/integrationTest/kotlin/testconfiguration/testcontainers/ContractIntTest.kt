package testconfiguration.testcontainers

import io.provenance.bilateral.client.BilateralContractClient
import io.provenance.bilateral.client.BilateralContractClientLogger
import io.provenance.bilateral.util.ContractAddressResolver
import io.provenance.client.grpc.GasEstimationMethod
import io.provenance.client.grpc.PbClient
import mu.KLogging
import org.testcontainers.containers.BindMode
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.util.BilateralSmartContractUtil
import testconfiguration.util.CoinFundingUtil
import testconfiguration.util.ContractInstantiationResult
import testconfiguration.util.ScopeWriteUtil
import java.net.URI
import java.util.TimeZone
import kotlin.system.exitProcess

abstract class ContractIntTest {
    companion object : KLogging() {
        private var containerIsStarted: Boolean = false

        private val container: ProvenanceTestContainer = ProvenanceTestContainer()
            .withNetworkAliases("provenance")
            .withClasspathResourceMapping("data/provenance", "/home/provenance_seed", BindMode.READ_ONLY)
            .withExposedPorts(1317, 9090, 26657)
            .withCommand("bash", "-c", "cp -rn /home/provenance_seed/* /home/provenance && /usr/bin/provenanced -t --home /home/provenance start")
            .waitingFor(ProvenanceWaitStrategy(BilateralAccounts.fundingAccount.address()))

        val pbClient: PbClient by lazy {
            PbClient(
                chainId = "chain-local",
                channelUri = URI.create("http://${container.host}:${container.getMappedPort(9090)}"),
                gasEstimationMethod = GasEstimationMethod.MSG_FEE_CALCULATION,
            )
        }

        lateinit var contractInfo: ContractInstantiationResult

        val bilateralClient by lazy {
            BilateralContractClient.builder(
                pbClient = pbClient,
                addressResolver = ContractAddressResolver.ProvidedAddress(contractInfo.contractAddress)
            ).setLogger(
                BilateralContractClientLogger.Custom(
                    infoLogger = { message -> logger.info(message) },
                    errorLogger = { message, e -> logger.error(message, e) },
                )
            ).build()
        }
    }

    init {
        if (!containerIsStarted) {
            try {
                container.start()
            } catch (e: Exception) {
                logger.error("Failed to start provenance container", e)
                exitProcess(1)
            }
            try {
                logger.info("Normalizing timezone to UTC to ensure deserialized values match in tests")
                TimeZone.setDefault(TimeZone.getTimeZone("UTC"))
                CoinFundingUtil.fundAccounts(
                    pbClient = pbClient,
                    senderAccount = BilateralAccounts.fundingAccount,
                    receiverAccounts = listOf(
                        BilateralAccounts.adminAccount,
                        BilateralAccounts.askerAccount,
                        BilateralAccounts.bidderAccount
                    ),
                )
                logger.info("Setting up the local bilateral exchange smart contract")
                contractInfo = BilateralSmartContractUtil.instantiateSmartContract(pbClient, BilateralAccounts.adminAccount)
                logger.info("Successfully established the contract with name [${contractInfo.contractBindingName}] at address [${contractInfo.contractAddress}]")
                ScopeWriteUtil.writeInitialScopeSpecs(pbClient)
            } catch (e: Exception) {
                logger.error("Failed to fund accounts and/or stand up the exchange smart contract", e)
                exitProcess(1)
            }
            containerIsStarted = true
        }
    }
}
