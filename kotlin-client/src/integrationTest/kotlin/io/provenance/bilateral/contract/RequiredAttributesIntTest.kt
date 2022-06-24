package io.provenance.bilateral.contract

import cosmos.base.v1beta1.CoinOuterClass.Coin
import cosmos.tx.v1beta1.ServiceOuterClass
import io.provenance.attribute.v1.AttributeType
import io.provenance.attribute.v1.MsgAddAttributeRequest
import io.provenance.bilateral.client.BroadcastOptions
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.AttributeRequirement
import io.provenance.bilateral.models.AttributeRequirementType
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.client.grpc.BaseReqSigner
import io.provenance.client.grpc.Signer
import io.provenance.client.protobuf.extensions.toAny
import io.provenance.client.protobuf.extensions.toTxBody
import io.provenance.scope.util.toByteString
import mu.KLogging
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.checkIsSuccess
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.assertSucceeds
import testconfiguration.functions.bindNamesToSigner
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.time.OffsetDateTime
import java.util.UUID
import kotlin.test.assertEquals
import kotlin.test.assertFails

class RequiredAttributesIntTest : ContractIntTest() {
    private companion object : KLogging() {
        private val DEFAULT_ATTRIBUTES: List<String> = listOf("first.pb", "second.pb", "third.pb")
        private val DEFAULT_QUOTE: List<Coin> = newCoins(100, "nhash")
        private val DEFAULT_BASE: List<Coin> = newCoins(100, "nhash")
    }

    @Test
    fun testRequiredAttributesTypeAll() {
        bindNamesToSigner(
            pbClient = pbClient,
            names = DEFAULT_ATTRIBUTES,
            signer = BilateralAccounts.adminAccount,
            restricted = true,
        )
        val askUuid = UUID.randomUUID()
        logger.info("Creating an ask with UUID: $askUuid, requiring all attributes: $DEFAULT_ATTRIBUTES")
        createAndSendAsk(askUuid, DEFAULT_ATTRIBUTES, AttributeRequirementType.ALL)
        val bidUuid = UUID.randomUUID()
        logger.info("Creating bid with UUID: $bidUuid, requiring all attributes: $DEFAULT_ATTRIBUTES")
        createAndSendBid(bidUuid, DEFAULT_ATTRIBUTES, AttributeRequirementType.ALL)
        val executeMatch = ExecuteMatch.new(
            askId = askUuid.toString(),
            bidId = bidUuid.toString(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting a failure")
        assertFails("Expected the match to fail because the asker and bidder are both missing the attributes") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        addDummyAttributesToAddress(
            attributeOwner = BilateralAccounts.adminAccount,
            attributes = DEFAULT_ATTRIBUTES,
            targetAddress = BilateralAccounts.askerAccount.address(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting a failure")
        assertFails("Expected the match to fail because the bidder is still missing the attributes") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        addDummyAttributesToAddress(
            attributeOwner = BilateralAccounts.adminAccount,
            attributes = DEFAULT_ATTRIBUTES,
            targetAddress = BilateralAccounts.bidderAccount.address(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting success")
        assertSucceeds("Expected match to succeed now that all required attributes are held by ask and bid") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }

    @Test
    fun testRequiredAttributesTypeAny() {
        bindNamesToSigner(
            pbClient = pbClient,
            names = DEFAULT_ATTRIBUTES,
            signer = BilateralAccounts.adminAccount,
            restricted = true,
        )
        val askUuid = UUID.randomUUID()
        logger.info("Creating an ask with UUID: $askUuid, requiring any of attributes: $DEFAULT_ATTRIBUTES")
        createAndSendAsk(askUuid, DEFAULT_ATTRIBUTES, AttributeRequirementType.ANY)
        val bidUuid = UUID.randomUUID()
        println("Creating bid with UUID: $bidUuid, requiring any of attributes: $DEFAULT_ATTRIBUTES")
        createAndSendBid(bidUuid, DEFAULT_ATTRIBUTES, AttributeRequirementType.ANY)
        val executeMatch = ExecuteMatch.new(
            askId = askUuid.toString(),
            bidId = bidUuid.toString(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting a failure")
        assertFails("Expected the match to fail because the asker and bidder are both missing the attributes") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        // Only add a random one of the attributes to the asker account to spice things up and verify that only one of
        // any of the values is required
        addDummyAttributesToAddress(
            attributeOwner = BilateralAccounts.adminAccount,
            attributes = DEFAULT_ATTRIBUTES.random().let(::listOf),
            targetAddress = BilateralAccounts.askerAccount.address(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting a failure")
        assertFails("Expected the match to fail because the bidder is still missing attributes") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        // Only add a random one of the attributes to the bidder account to spice things up and verify that only one of
        // any of the values is required
        addDummyAttributesToAddress(
            attributeOwner = BilateralAccounts.adminAccount,
            attributes = DEFAULT_ATTRIBUTES.random().let(::listOf),
            targetAddress = BilateralAccounts.bidderAccount.address(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting success")
        assertSucceeds("Expected the match to succeed now that both ask and bid have one attribute") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }

    private fun addDummyAttributesToAddress(
        attributeOwner: Signer,
        attributes: List<String>,
        targetAddress: String,
    ) {
        attributes.map { attributeName ->
            MsgAddAttributeRequest.newBuilder().also { addAttribute ->
                addAttribute.account = targetAddress
                addAttribute.attributeType = AttributeType.ATTRIBUTE_TYPE_STRING
                addAttribute.name = attributeName
                addAttribute.owner = attributeOwner.address()
                addAttribute.value = "dummyvalue".toByteString()
            }.build().toAny()
        }.also { attributeMsgs ->
            logger.info("Adding attributes $attributes to address [$targetAddress] with owner address [${attributeOwner.address()}]")
            pbClient.estimateAndBroadcastTx(
                txBody = attributeMsgs.toTxBody(),
                signers = listOf(BaseReqSigner(attributeOwner)),
                mode = ServiceOuterClass.BroadcastMode.BROADCAST_MODE_BLOCK,
                gasAdjustment = 1.3,
            ).checkIsSuccess()
        }
    }

    private fun createAndSendAsk(
        askUuid: UUID,
        attributes: List<String>,
        requirementType: AttributeRequirementType,
    ) {
        val createAsk = CreateAsk.newCoinTrade(
            id = askUuid.toString(),
            quote = DEFAULT_QUOTE,
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(attributes, requirementType),
            )
        )
        bilateralClient.createAsk(
            createAsk = createAsk,
            signer = BilateralAccounts.askerAccount,
            options = BroadcastOptions(funds = DEFAULT_BASE),
        )
        val askOrder = bilateralClient.assertAskExists(askUuid.toString())
        assertEquals(
            expected = createAsk.createAsk.descriptor?.effectiveTime,
            actual = askOrder.descriptor?.effectiveTime,
            message = "Expected the effective time to correctly deserialize from the contract response",
        )
    }

    private fun createAndSendBid(
        bidUuid: UUID,
        attributes: List<String>,
        requirementType: AttributeRequirementType,
    ) {
        val createBid = CreateBid.newCoinTrade(
            id = bidUuid.toString(),
            base = DEFAULT_BASE,
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(attributes, requirementType),
            )
        )
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
            options = BroadcastOptions(funds = DEFAULT_QUOTE),
        )
        val bidOrder = bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = createBid.createBid.descriptor?.effectiveTime,
            actual = bidOrder.descriptor?.effectiveTime,
            message = "Expected the effective time to correctly deserialize from the contract response",
        )
    }
}
