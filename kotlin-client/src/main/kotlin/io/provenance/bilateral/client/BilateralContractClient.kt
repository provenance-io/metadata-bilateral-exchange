package io.provenance.bilateral.client

import com.fasterxml.jackson.core.type.TypeReference
import com.fasterxml.jackson.databind.ObjectMapper
import cosmos.base.v1beta1.CoinOuterClass.Coin
import cosmos.tx.v1beta1.ServiceOuterClass.BroadcastTxResponse
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

    fun searchAsks(searchAsks: SearchAsks): ContractSearchResult<AskOrder> = queryContractWithReference(
        query = searchAsks,
        typeReference = object : TypeReference<ContractSearchResult<AskOrder>>() {},
    )

    fun searchAsksOrNull(searchAsks: SearchAsks): ContractSearchResult<AskOrder>? = tryOrNull { searchAsks(searchAsks) }

    fun searchBids(searchBids: SearchBids): ContractSearchResult<BidOrder> = queryContractWithReference(
        query = searchBids,
        typeReference = object : TypeReference<ContractSearchResult<BidOrder>>() {},
    )

    fun searchBidsOrNull(searchBids: SearchBids): ContractSearchResult<BidOrder>? = tryOrNull { searchBids(searchBids) }

    fun getContractInfo(): ContractInfo = queryContract(GetContractInfo.new())

    fun getContractInfoOrNull(): ContractInfo? = tryOrNull { getContractInfo() }

    fun createAsk(
        createAsk: CreateAsk,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions()
    ): BroadcastTxResponse = executeContract(createAsk, signer, options, funds = createAsk.getFunds())

    fun createBid(
        createBid: CreateBid,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions()
    ): BroadcastTxResponse = executeContract(createBid, signer, options, funds = createBid.getFunds())

    fun cancelAsk(
        cancelAsk: CancelAsk,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions()
    ): BroadcastTxResponse = executeContract(cancelAsk, signer, options, funds = emptyList())

    fun cancelBid(
        cancelBid: CancelBid,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions()
    ): BroadcastTxResponse = executeContract(cancelBid, signer, options, funds = emptyList())

    // IMPORTANT: The Signer used in this function must be the contract's admin account.  This value can be found by
    // running getContractInfo()
    fun executeMatch(
        executeMatch: ExecuteMatch,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions()
    ): BroadcastTxResponse = executeContract(executeMatch, signer, options, funds = emptyList())

    fun generateCreateAskMsg(
        createAsk: CreateAsk,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(createAsk, senderAddress, funds = createAsk.getFunds())

    fun generateCreateBidMsg(
        createBid: CreateBid,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(createBid, senderAddress, funds = createBid.getFunds())

    fun generateCancelAskMsg(
        cancelAsk: CancelAsk,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(cancelAsk, senderAddress, funds = emptyList())

    fun generateCancelBidMsg(
        cancelBid: CancelBid,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(cancelBid, senderAddress, funds = emptyList())

    fun generateExecuteMatchMsg(
        executeMatch: ExecuteMatch,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(executeMatch, senderAddress, funds = emptyList())

    /**
     * Converts a class that inherits from ContractExecuteMsg to a MsgExecuteContract.  This ensures
     */
    private fun generateProtoExecuteMsg(
        executeMsg: ContractExecuteMsg,
        senderAddress: String,
        funds: List<Coin>,
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
        funds: List<Coin>,
    ): BroadcastTxResponse {
        val msg = generateProtoExecuteMsg(executeMsg, signer.address(), funds)
        return pbClient.estimateAndBroadcastTx(
            txBody = msg.toAny().toTxBody(),
            signers = listOf(
                BaseReqSigner(
                    signer = signer,
                    sequenceOffset = options.sequenceOffset,
                    account = options.baseAccount
                )
            ),
            mode = options.broadcastMode,
            gasAdjustment = options.gasAdjustment,
        ).also { response ->
            if (response.txResponse.code != 0) {
                throw IllegalStateException("FAILED: ${response.txResponse.rawLog}")
            }
        }
    }

    private fun <T : ContractQueryMsg> getQueryResponseBytes(query: T): ByteArray = pbClient.wasmClient.queryWasm(
        QueryOuterClass.QuerySmartContractStateRequest.newBuilder().also { req ->
            req.address = contractAddress
            req.queryData = query.toJsonByteString(objectMapper)
        }.build()
    ).data.toByteArray()

    private inline fun <T : ContractQueryMsg, reified U : Any> queryContract(query: T): U =
        getQueryResponseBytes(query).let { bytes -> objectMapper.readValue(bytes, U::class.java) }

    /**
     * Allows a type reference to be passed in, ensuring that generic types within response values can be properly
     * deserialized by Jackson using simple byte input.
     */
    private fun <T : ContractQueryMsg, U : Any> queryContractWithReference(
        query: T,
        typeReference: TypeReference<U>,
    ): U = getQueryResponseBytes(query).let { bytes -> objectMapper.readValue(bytes, typeReference) }
}
