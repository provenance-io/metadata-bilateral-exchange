package testconfiguration.testcontainers

import io.provenance.bilateral.client.BilateralContractClient
import io.provenance.bilateral.util.ContractAddressResolver
import io.provenance.client.grpc.GasEstimationMethod
import io.provenance.client.grpc.PbClient
import mu.KLogging
import org.junit.jupiter.api.BeforeEach
import org.testcontainers.containers.BindMode
import org.testcontainers.junit.jupiter.Container
import org.testcontainers.junit.jupiter.Testcontainers
import java.net.URI
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.util.BilateralSmartContractUtil
import testconfiguration.util.CoinFundingUtil
import testconfiguration.util.ContractInstantiationResult
import java.util.TimeZone

@Testcontainers
abstract class ContractIntTest {
    private companion object : KLogging()

    @Container
    val provenanceContainer: ProvenanceTestContainer = ProvenanceTestContainer()
        .withNetworkAliases("provenance")
        .withClasspathResourceMapping("data/provenance", "/home/provenance_seed", BindMode.READ_ONLY)
        .withExposedPorts(1317, 9090, 26657)
        .withCommand("bash", "-c", "cp -rn /home/provenance_seed/* /home/provenance && /usr/bin/provenanced -t --home /home/provenance start")
        .waitingFor(ProvenanceWaitStrategy(BilateralAccounts.fundingAccount.address()))

    val pbClient: PbClient by lazy {
        PbClient(
            chainId = "chain-local",
            channelUri = URI.create("http://${provenanceContainer.host}:${provenanceContainer.getMappedPort(9090)}"),
            gasEstimationMethod = GasEstimationMethod.MSG_FEE_CALCULATION,
        )
    }

    lateinit var contractInfo: ContractInstantiationResult

    val bilateralClient by lazy {
        BilateralContractClient.new(
            pbClient = pbClient,
            addressResolver = ContractAddressResolver.ProvidedAddress(contractInfo.contractAddress)
        )
    }

    @BeforeEach
    fun beforeEachTest() {
        logger.info("Normalizing timezone to UTC to ensure deserialized values match in tests")
        TimeZone.setDefault(TimeZone.getTimeZone("UTC"))
        CoinFundingUtil.fundAccounts(
            pbClient = pbClient,
            senderAccount = BilateralAccounts.fundingAccount,
            receiverAccounts = listOf(BilateralAccounts.adminAccount, BilateralAccounts.askerAccount, BilateralAccounts.bidderAccount),
        )
        logger.info("Setting up the local bilateral exchange smart contract")
        contractInfo = BilateralSmartContractUtil.instantiateSmartContract(pbClient, BilateralAccounts.adminAccount)
        logger.info("Successfully established the contract with name [${contractInfo.contractBindingName}] at address [${contractInfo.contractAddress}]")
    }
}
