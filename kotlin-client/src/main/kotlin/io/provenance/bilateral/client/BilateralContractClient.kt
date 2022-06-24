package io.provenance.bilateral.client

import com.fasterxml.jackson.databind.ObjectMapper
import cosmos.base.v1beta1.CoinOuterClass.Coin
import cosmwasm.wasm.v1.QueryOuterClass
import cosmwasm.wasm.v1.Tx.MsgExecuteContract
import io.provenance.bilateral.execute.CancelAsk
import io.provenance.bilateral.execute.CancelBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.functions.tryOrNull
import io.provenance.bilateral.interfaces.ContractExecuteMsg
import io.provenance.bilateral.interfaces.ContractQueryMsg
import io.provenance.bilateral.models.AskOrder
import io.provenance.bilateral.models.BidOrder
import io.provenance.bilateral.models.ContractInfo
import io.provenance.bilateral.query.ContractSearchResult
import io.provenance.bilateral.query.GetAsk
import io.provenance.bilateral.query.GetBid
import io.provenance.bilateral.query.GetContractInfo
import io.provenance.bilateral.query.SearchAsks
import io.provenance.bilateral.query.SearchBids
import io.provenance.bilateral.util.ContractAddressResolver
import io.provenance.bilateral.util.ObjectMapperProvider
import io.provenance.client.grpc.BaseReqSigner
import io.provenance.client.grpc.PbClient
import io.provenance.client.grpc.Signer
import io.provenance.client.protobuf.extensions.queryWasm
import io.provenance.client.protobuf.extensions.toAny
import io.provenance.client.protobuf.extensions.toTxBody

class BilateralContractClient private constructor(
    val pbClient: PbClient,
    val objectMapper: ObjectMapper,
    private val addressResolver: ContractAddressResolver,
) {
    companion object {
        fun new(
            pbClient: PbClient,
            addressResolver: ContractAddressResolver,
            objectMapper: ObjectMapper = ObjectMapperProvider.OBJECT_MAPPER,
        ): BilateralContractClient = BilateralContractClient(
            pbClient = pbClient,
            objectMapper = objectMapper,
            addressResolver = addressResolver,
        )
    }

    val contractAddress by lazy { addressResolver.getAddress(pbClient) }

    fun getAsk(id: String): AskOrder = queryContract(GetAsk.new(id))

    fun getAskOrNull(id: String): AskOrder? = tryOrNull { getAsk(id) }

    fun getBid(id: String): BidOrder = queryContract(GetBid.new(id))

    fun getBidOrNull(id: String): BidOrder? = tryOrNull { getBid(id) }

    fun searchAsks(searchAsks: SearchAsks): ContractSearchResult<AskOrder> = queryContract(searchAsks)

    fun searchAsksOrNull(searchAsks: SearchAsks): ContractSearchResult<AskOrder>? = tryOrNull { searchAsks(searchAsks) }

    fun searchBids(searchBids: SearchBids): ContractSearchResult<BidOrder> = queryContract(searchBids)

    fun searchBidsOrNull(searchBids: SearchBids): ContractSearchResult<BidOrder>? = tryOrNull { searchBids(searchBids) }

    fun getContractInfo(): ContractInfo = queryContract(GetContractInfo.new())

    fun getContractInfoOrNull(): ContractInfo? = tryOrNull { getContractInfo() }

    fun createAsk(createAsk: CreateAsk, signer: Signer, options: BroadcastOptions = BroadcastOptions()) {
        executeContract(createAsk, signer, options)
    }

    fun createBid(createBid: CreateBid, signer: Signer, options: BroadcastOptions = BroadcastOptions()) {
        executeContract(createBid, signer, options)
    }

    fun cancelAsk(cancelAsk: CancelAsk, signer: Signer, options: BroadcastOptions = BroadcastOptions()) {
        executeContract(cancelAsk, signer, options)
    }

    fun cancelBid(cancelBid: CancelBid, signer: Signer, options: BroadcastOptions = BroadcastOptions()) {
        executeContract(cancelBid, signer, options)
    }

    // IMPORTANT: The Signer used in this function must be the contract's admin account.  This value can be found by
    // running getContractInfo()
    fun executeMatch(executeMatch: ExecuteMatch, signer: Signer, options: BroadcastOptions = BroadcastOptions()) {
        executeContract(executeMatch, signer, options)
    }

    fun generateCreateAskMsg(createAsk: CreateAsk, senderAddress: String, funds: List<Coin> = emptyList()) {
        generateProtoExecuteMsg(createAsk, senderAddress, funds)
    }

    fun generateCreateBidMsg(createBid: CreateBid, senderAddress: String, funds: List<Coin> = emptyList()) {
        generateProtoExecuteMsg(createBid, senderAddress, funds)
    }

    fun generateCancelAskMsg(cancelAsk: CancelAsk, senderAddress: String, funds: List<Coin> = emptyList()) {
        generateProtoExecuteMsg(cancelAsk, senderAddress, funds)
    }

    fun generateCancelBidMsg(cancelBid: CancelBid, senderAddress: String, funds: List<Coin> = emptyList()) {
        generateProtoExecuteMsg(cancelBid, senderAddress, funds)
    }

    fun generateExecuteMatchMsg(executeMatch: ExecuteMatch, senderAddress: String, funds: List<Coin> = emptyList()) {
        generateProtoExecuteMsg(executeMatch, senderAddress, funds)
    }

    /**
     * Converts a class that inherits from ContractExecuteMsg to a MsgExecuteContract.  This ensures
     */
    private fun generateProtoExecuteMsg(
        executeMsg: ContractExecuteMsg,
        senderAddress: String,
        funds: List<Coin> = emptyList()
    ): MsgExecuteContract = executeMsg.toExecuteMsg(
        objectMapper = objectMapper,
        contractAddress = contractAddress,
        senderBech32Address = senderAddress,
        funds = funds,
    )

    /**
     * Sends a ContractExecuteMsg to the resolved Metadata Bilateral Exchange contract address with the specified funds.
     * Throws exceptions if the PbClient is misconfigured or if a Provenance Blockchain or smart contract error occurs.
     */
    private fun executeContract(
        executeMsg: ContractExecuteMsg,
        signer: Signer,
        options: BroadcastOptions,
    ) {
        val msg = generateProtoExecuteMsg(executeMsg, signer.address(), options.funds)
        pbClient.estimateAndBroadcastTx(
            txBody = msg.toAny().toTxBody(),
            signers = listOf(BaseReqSigner(
                signer = signer,
                sequenceOffset = options.sequenceOffset,
                account = options.baseAccount
            )),
            mode = options.broadcastMode,
            gasAdjustment = options.gasAdjustment,
        ).also { response ->
            if (response.txResponse.code != 0) {
                throw IllegalStateException("FAILED: ${response.txResponse.rawLog}")
            }
        }
    }

    private inline fun <T : ContractQueryMsg, reified U : Any> queryContract(query: T): U =
        pbClient.wasmClient.queryWasm(
            QueryOuterClass.QuerySmartContractStateRequest.newBuilder().also { req ->
                req.address = contractAddress
                req.queryData = query.toJsonByteString(objectMapper)
            }.build()
        ).data.toByteArray().let { bytes -> objectMapper.readValue(bytes, U::class.java) }
}
