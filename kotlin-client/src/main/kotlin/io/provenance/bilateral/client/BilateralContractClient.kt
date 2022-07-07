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
import io.provenance.bilateral.execute.UpdateSettings
import io.provenance.bilateral.functions.tryOrNull
import io.provenance.bilateral.interfaces.ContractExecuteMsg
import io.provenance.bilateral.interfaces.ContractQueryMsg
import io.provenance.bilateral.models.AskOrder
import io.provenance.bilateral.models.BidOrder
import io.provenance.bilateral.models.ContractInfo
import io.provenance.bilateral.models.MatchReport
import io.provenance.bilateral.query.ContractSearchResult
import io.provenance.bilateral.query.GetAsk
import io.provenance.bilateral.query.GetAskByCollateralId
import io.provenance.bilateral.query.GetBid
import io.provenance.bilateral.query.GetContractInfo
import io.provenance.bilateral.query.GetMatchReport
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

    fun getAsk(id: String): AskOrder = queryContract(GetAsk(id))

    fun getAskOrNull(id: String): AskOrder? = tryOrNull { getAsk(id) }

    fun getAskByCollateralId(collateralId: String): AskOrder = queryContract(GetAskByCollateralId(collateralId))

    fun getAskByCollateralIdOrNull(collateralId: String): AskOrder? = tryOrNull { getAskByCollateralId(collateralId) }

    fun getBid(id: String): BidOrder = queryContract(GetBid(id))

    fun getBidOrNull(id: String): BidOrder? = tryOrNull { getBid(id) }

    fun getMatchReport(askId: String, bidId: String): MatchReport = queryContract(GetMatchReport(askId, bidId))

    fun getMatchReportOrNull(askId: String, bidId: String): MatchReport? = tryOrNull { getMatchReport(askId, bidId) }

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

    fun getContractInfo(): ContractInfo = queryContract(GetContractInfo())

    fun getContractInfoOrNull(): ContractInfo? = tryOrNull { getContractInfo() }

    fun createAsk(
        createAsk: CreateAsk,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions(),
    ): BroadcastTxResponse = executeContract(
        executeMsg = createAsk,
        signer = signer,
        options = options,
        funds = createAsk.getFunds(askFee = this.getContractInfo().askFee),
    )

    fun createBid(
        createBid: CreateBid,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions(),
    ): BroadcastTxResponse = executeContract(
        executeMsg = createBid,
        signer = signer,
        options = options,
        funds = createBid.getFunds(bidFee = this.getContractInfo().bidFee),
    )

    fun cancelAsk(
        askId: String,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions(),
    ): BroadcastTxResponse = executeContract(
        executeMsg = CancelAsk(askId),
        signer = signer,
        options = options,
        funds = emptyList(),
    )

    fun cancelBid(
        bidId: String,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions(),
    ): BroadcastTxResponse = executeContract(
        executeMsg = CancelBid(bidId),
        signer = signer,
        options = options,
        funds = emptyList(),
    )

    // IMPORTANT: The Signer used in this function must be the contract's admin account or the asker associated with the
    // match message's askId.
    fun executeMatch(
        executeMatch: ExecuteMatch,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions(),
    ): BroadcastTxResponse = executeContract(
        executeMsg = executeMatch,
        signer = signer,
        options = options,
        funds = emptyList(),
    )

    // IMPORTANT: The Signer used in this function must be the contract's admin account.
    fun updateSettings(
        updateSettings: UpdateSettings,
        signer: Signer,
        options: BroadcastOptions = BroadcastOptions(),
    ): BroadcastTxResponse = executeContract(
        executeMsg = updateSettings,
        signer = signer,
        options = options,
        funds = emptyList(),
    )

    fun generateCreateAskMsg(
        createAsk: CreateAsk,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(
        executeMsg = createAsk,
        senderAddress = senderAddress,
        funds = createAsk.getFunds(askFee = this.getContractInfo().askFee),
    )

    fun generateCreateBidMsg(
        createBid: CreateBid,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(
        executeMsg = createBid,
        senderAddress = senderAddress,
        funds = createBid.getFunds(bidFee = this.getContractInfo().bidFee),
    )

    fun generateCancelAskMsg(
        askId: String,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(
        executeMsg = CancelAsk(askId),
        senderAddress = senderAddress,
        funds = emptyList(),
    )

    fun generateCancelBidMsg(
        bidId: String,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(
        executeMsg = CancelBid(bidId),
        senderAddress = senderAddress,
        funds = emptyList(),
    )

    // IMPORTANT: The Signer used in this function must be the contract's admin account or the asker associated with the
    // match message's askId.
    fun generateExecuteMatchMsg(
        executeMatch: ExecuteMatch,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(
        executeMsg = executeMatch,
        senderAddress = senderAddress,
        funds = emptyList(),
    )

    // IMPORTANT: The Signer used in this function must be the contract's admin account.
    fun generateUpdateSettingsMsg(
        updateSettings: UpdateSettings,
        senderAddress: String,
    ): MsgExecuteContract = generateProtoExecuteMsg(
        executeMsg = updateSettings,
        senderAddress = senderAddress,
        funds = emptyList(),
    )

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
