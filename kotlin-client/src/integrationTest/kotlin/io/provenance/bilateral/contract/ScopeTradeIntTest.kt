package io.provenance.bilateral.contract

import cosmos.tx.v1beta1.ServiceOuterClass
import io.provenance.bilateral.client.BroadcastOptions
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.models.RequestDescriptor
import io.provenance.client.grpc.BaseReqSigner
import io.provenance.client.protobuf.extensions.toAny
import io.provenance.client.protobuf.extensions.toTxBody
import io.provenance.metadata.v1.MsgWriteScopeRequest
import io.provenance.metadata.v1.Party
import io.provenance.metadata.v1.PartyType
import io.provenance.scope.util.MetadataAddress
import io.provenance.scope.util.toByteString
import io.provenance.spec.HELOCSpecification
import mu.KLogging
import org.junit.jupiter.api.Test
import testconfiguration.accounts.BilateralAccounts
import testconfiguration.extensions.checkIsSuccess
import testconfiguration.functions.assertAskExists
import testconfiguration.functions.assertAskIsDeleted
import testconfiguration.functions.assertBidExists
import testconfiguration.functions.assertBidIsDeleted
import testconfiguration.functions.newCoins
import testconfiguration.testcontainers.ContractIntTest
import java.time.OffsetDateTime
import java.util.UUID

class ScopeTradeIntTest : ContractIntTest() {
    private companion object : KLogging()

    @Test
    fun testScopeTrade() {
        val scopeUuid = UUID.randomUUID()
        val writeSpecMsgs = HELOCSpecification.specificationMsgs(BilateralAccounts.adminAccount.address())
        logger.info("Writing HELOC asset type specification messages, owned by the contract admin: ${BilateralAccounts.adminAccount.address()}")
        pbClient.estimateAndBroadcastTx(
            txBody = writeSpecMsgs.map { it.toAny() }.toTxBody(),
            signers = BilateralAccounts.adminAccount.let(::BaseReqSigner).let(::listOf),
            mode = ServiceOuterClass.BroadcastMode.BROADCAST_MODE_BLOCK,
            gasAdjustment = 1.3,
        ).checkIsSuccess()
        val writeScopeMsg = MsgWriteScopeRequest.newBuilder().also { req ->
            req.scopeUuid  = scopeUuid.toString()
            req.specUuid = HELOCSpecification.scopeSpecConfig.id.toString()
            req.addSigners(BilateralAccounts.askerAccount.address())
            req.scopeBuilder.scopeId = MetadataAddress.forScope(scopeUuid).bytes.toByteString()
            req.scopeBuilder.specificationId = MetadataAddress.forScopeSpecification(HELOCSpecification.scopeSpecConfig.id).bytes.toByteString()
            req.scopeBuilder.valueOwnerAddress = contractInfo.contractAddress
            req.scopeBuilder.addOwners(Party.newBuilder().also { party ->
                party.address = contractInfo.contractAddress
                party.role = PartyType.PARTY_TYPE_OWNER
            })
            req.scopeBuilder.addDataAccess(BilateralAccounts.askerAccount.address())
        }.build().toAny()
        logger.info("Creating scope with UUID [$scopeUuid] owned by the contract")
        pbClient.estimateAndBroadcastTx(
            txBody = writeScopeMsg.toTxBody(),
            signers = BilateralAccounts.askerAccount.let(::BaseReqSigner).let(::listOf),
            mode = ServiceOuterClass.BroadcastMode.BROADCAST_MODE_BLOCK,
        ).checkIsSuccess()
        val askUuid = UUID.randomUUID()
        val createAsk = CreateAsk.newScopeTrade(
            id = askUuid.toString(),
            scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
            quote = newCoins(50000, "nhash"),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        logger.info("Creating scope trade ask [$askUuid]")
        bilateralClient.createAsk(createAsk, BilateralAccounts.askerAccount)
        bilateralClient.assertAskExists(askUuid.toString())
        val bidUuid = UUID.randomUUID()
        val createBid = CreateBid.newScopeTrade(
            id = bidUuid.toString(),
            scopeAddress = MetadataAddress.forScope(scopeUuid).toString(),
            descriptor = RequestDescriptor("Example description", OffsetDateTime.now()),
        )
        logger.info("Creating scope trade bid [$bidUuid]")
        bilateralClient.createBid(
            createBid = createBid,
            signer = BilateralAccounts.bidderAccount,
            options = BroadcastOptions(funds = newCoins(50000, "nhash")),
        )
        bilateralClient.assertBidExists(bidUuid.toString())
        val executeMatch = ExecuteMatch.new(askUuid.toString(), bidUuid.toString())
        logger.info("Executing match for ask [$askUuid] and bid [$bidUuid]")
        bilateralClient.executeMatch(executeMatch, BilateralAccounts.adminAccount)
        bilateralClient.assertAskIsDeleted(askUuid.toString())
        bilateralClient.assertBidIsDeleted(bidUuid.toString())
    }
}
