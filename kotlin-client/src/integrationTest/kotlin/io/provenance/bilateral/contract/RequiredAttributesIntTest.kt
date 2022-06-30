package io.provenance.bilateral.contract

import cosmos.base.v1beta1.CoinOuterClass.Coin
import cosmos.tx.v1beta1.ServiceOuterClass
import io.provenance.attribute.v1.AttributeType
import io.provenance.attribute.v1.MsgAddAttributeRequest
import io.provenance.bilateral.execute.CancelAsk
import io.provenance.bilateral.execute.CancelBid
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
        private val DEFAULT_QUOTE: List<Coin> = newCoins(100, "nhash")
        private val DEFAULT_BASE: List<Coin> = newCoins(100, "nhash")
    }

    @Test
    fun testRequiredAttributesTypeAll() {
        val attributePrefix = "testRequiredAttributesTypeAll".lowercase()
        val testAttributes = listOf("${attributePrefix}a.pb", "${attributePrefix}b.pb", "${attributePrefix}c.pb")
        bindNamesToSigner(
            pbClient = pbClient,
            names = testAttributes,
            signer = BilateralAccounts.adminAccount,
            restricted = true,
        )
        val askUuid = UUID.randomUUID()
        logger.info("Creating an ask with UUID: $askUuid, requiring all attributes: $testAttributes")
        createAndSendAsk(askUuid, testAttributes, AttributeRequirementType.ALL)
        val bidUuid = UUID.randomUUID()
        logger.info("Creating bid with UUID: $bidUuid, requiring all attributes: $testAttributes")
        createAndSendBid(bidUuid, testAttributes, AttributeRequirementType.ALL)
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
            attributes = testAttributes,
            targetAddress = BilateralAccounts.askerAccount.address(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting a failure")
        assertFails("Expected the match to fail because the bidder is still missing the attributes") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        addDummyAttributesToAddress(
            attributeOwner = BilateralAccounts.adminAccount,
            attributes = testAttributes,
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
        val attributePrefix = "testRequiredAttributesTypeAny".lowercase()
        val testAttributes = listOf("${attributePrefix}a.pb", "${attributePrefix}b.pb", "${attributePrefix}c.pb")
        bindNamesToSigner(
            pbClient = pbClient,
            names = testAttributes,
            signer = BilateralAccounts.adminAccount,
            restricted = true,
        )
        val askUuid = UUID.randomUUID()
        logger.info("Creating an ask with UUID: $askUuid, requiring any of attributes: $testAttributes")
        createAndSendAsk(askUuid, testAttributes, AttributeRequirementType.ANY)
        val bidUuid = UUID.randomUUID()
        logger.info("Creating bid with UUID: $bidUuid, requiring any of attributes: $testAttributes")
        createAndSendBid(bidUuid, testAttributes, AttributeRequirementType.ANY)
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
            attributes = testAttributes.random().let(::listOf),
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
            attributes = testAttributes.random().let(::listOf),
            targetAddress = BilateralAccounts.bidderAccount.address(),
        )
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid] and expecting success")
        assertSucceeds("Expected the match to succeed now that both ask and bid have one attribute") {
            bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        }
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }

    @Test
    fun testRequiredAttributesTypeNone() {
        val attributePrefix = "testRequiredAttributesTypeNone".lowercase()
        val testAttributes = listOf("${attributePrefix}a.pb", "${attributePrefix}b.pb", "${attributePrefix}c.pb")
        bindNamesToSigner(
            pbClient = pbClient,
            names = testAttributes,
            signer = BilateralAccounts.adminAccount,
            restricted = true,
        )
        val firstAskUuid = UUID.randomUUID()
        logger.info("Creating an ask with UUID: $firstAskUuid, requiring none of attributes: $testAttributes")
        createAndSendAsk(firstAskUuid, testAttributes, AttributeRequirementType.NONE)
        val firstBidUuid = UUID.randomUUID()
        logger.info("Creating bid with UUID: $firstBidUuid, requiring none of attributes: $testAttributes")
        createAndSendBid(firstBidUuid, testAttributes, AttributeRequirementType.NONE)
        val firstExecuteMatch = ExecuteMatch.new(
            askId = firstAskUuid.toString(),
            bidId = firstBidUuid.toString(),
        )
        logger.info("Executing match for ask [$firstAskUuid] and bid [$firstBidUuid]")
        assertSucceeds("Expecting the match to succeed because neither account has any of the specified attributes") {
            bilateralClient.executeMatch(firstExecuteMatch, BilateralAccounts.adminAccount)
        }
        bilateralClient.assertAskIsDeleted(firstAskUuid.toString())
        bilateralClient.assertBidIsDeleted(firstBidUuid.toString())
        val secondAskUuid = UUID.randomUUID()
        logger.info("Creating ask with uuid: $secondAskUuid, requiring none of attributes: $testAttributes")
        createAndSendAsk(secondAskUuid, testAttributes, AttributeRequirementType.NONE)
        val secondBidUuid = UUID.randomUUID()
        logger.info("Creating bid with uuid: $secondBidUuid, requiring none of attributes: $testAttributes")
        createAndSendBid(secondBidUuid, testAttributes, AttributeRequirementType.NONE)
        val secondExecuteMatch = ExecuteMatch.new(
            askId = secondAskUuid.toString(),
            bidId = secondBidUuid.toString(),
        )
        // Only add a random one of the attributes to the asker account to spice things up and verify that only one of
        // any of the values is required to cause a rejection
        addDummyAttributesToAddress(
            attributeOwner = BilateralAccounts.adminAccount,
            attributes = testAttributes.random().let(::listOf),
            targetAddress = BilateralAccounts.askerAccount.address(),
        )
        logger.info("Executing match for ask [$secondAskUuid] and bid [$secondBidUuid] and expecting a failure")
        assertFails("Expected the match to fail because the asker has one of the attributes") {
            bilateralClient.executeMatch(secondExecuteMatch, BilateralAccounts.adminAccount)
        }
        // Only add a random one of the attributes to the bidder account to spice things up and verify that only one of
        // any of the values is required to cause a rejection
        addDummyAttributesToAddress(
            attributeOwner = BilateralAccounts.adminAccount,
            attributes = testAttributes.random().let(::listOf),
            targetAddress = BilateralAccounts.bidderAccount.address(),
        )
        logger.info("Executing match for ask [$secondAskUuid] and bid [$secondBidUuid] and expecting a failure")
        assertFails("Expected the match to fail because both asker and bidder have attributes that are not allowed") {
            bilateralClient.executeMatch(secondExecuteMatch, BilateralAccounts.adminAccount)
        }
        bilateralClient.assertAskExists(secondAskUuid.toString(), "Expected the ask to exist because a match was never made")
        bilateralClient.assertBidExists(secondBidUuid.toString(), "Expected the bid to exist because a match was never made")
        bilateralClient.cancelAsk(CancelAsk.new(secondAskUuid.toString()), BilateralAccounts.askerAccount)
        bilateralClient.assertAskIsDeleted(secondAskUuid.toString())
        bilateralClient.cancelBid(CancelBid.new(secondBidUuid.toString()), BilateralAccounts.bidderAccount)
        bilateralClient.assertBidIsDeleted(secondBidUuid.toString())
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
            base = DEFAULT_BASE,
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(attributes, requirementType),
            )
        )
        bilateralClient.createAsk(
            createAsk = createAsk,
            signer = BilateralAccounts.askerAccount,
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
            quote = DEFAULT_QUOTE,
            descriptor = RequestDescriptor(
                description = "Example description",
                effectiveTime = OffsetDateTime.now(),
                attributeRequirement = AttributeRequirement.new(attributes, requirementType),
            )
        )
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
        )
        val bidOrder = bilateralClient.assertBidExists(bidUuid.toString())
        assertEquals(
            expected = createBid.createBid.descriptor?.effectiveTime,
            actual = bidOrder.descriptor?.effectiveTime,
            message = "Expected the effective time to correctly deserialize from the contract response",
        )
    }
}
