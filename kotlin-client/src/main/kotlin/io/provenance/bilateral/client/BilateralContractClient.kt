package io.provenance.bilateral.client

import com.fasterxml.jackson.core.type.TypeReference
import com.fasterxml.jackson.databind.ObjectMapper
import cosmos.base.v1beta1.CoinOuterClass.Coin
import cosmwasm.wasm.v1.QueryOuterClass
import cosmwasm.wasm.v1.Tx.MsgExecuteContract
import io.provenance.bilateral.exceptions.NullContractResultException
import io.provenance.bilateral.exceptions.ProvenanceEventParsingException
import io.provenance.bilateral.execute.CancelAsk
import io.provenance.bilateral.execute.CancelBid
import io.provenance.bilateral.execute.CreateAsk
import io.provenance.bilateral.execute.CreateBid
import io.provenance.bilateral.execute.ExecuteMatch
import io.provenance.bilateral.execute.UpdateSettings
import io.provenance.bilateral.extensions.attribute
import io.provenance.bilateral.extensions.attributeOrNull
import io.provenance.bilateral.extensions.singleWasmEvent
import io.provenance.bilateral.interfaces.ContractExecuteMsg
import io.provenance.bilateral.interfaces.ContractQueryMsg
import io.provenance.bilateral.models.AskOrder
import io.provenance.bilateral.models.BidOrder
import io.provenance.bilateral.models.ContractInfo
import io.provenance.bilateral.models.MatchReport
import io.provenance.bilateral.models.executeresponse.CancelAskResponse
import io.provenance.bilateral.models.executeresponse.CancelBidResponse
import io.provenance.bilateral.models.executeresponse.CreateAskResponse
import io.provenance.bilateral.models.executeresponse.CreateBidResponse
import io.provenance.bilateral.models.executeresponse.ExecuteMatchResponse
import io.provenance.bilateral.models.executeresponse.UpdateSettingsResponse
import io.provenance.bilateral.query.ContractSearchRequest
import io.provenance.bilateral.query.ContractSearchResult
import io.provenance.bilateral.query.GetAsk
import io.provenance.bilateral.query.GetAskByCollateralId
import io.provenance.bilateral.query.GetBid
import io.provenance.bilateral.query.GetContractInfo
import io.provenance.bilateral.query.GetMatchReport
import io.provenance.bilateral.util.ContractAddressResolver
import io.provenance.bilateral.util.ObjectMapperProvider
import io.provenance.client.grpc.BaseReqSigner
import io.provenance.client.grpc.PbClient
import io.provenance.client.grpc.Signer
import io.provenance.client.protobuf.extensions.queryWasm
import io.provenance.client.protobuf.extensions.toAny
import io.provenance.client.protobuf.extensions.toTxBody
import org.bouncycastle.util.encoders.Hex
import tendermint.abci.Types.Event

class BilateralContractClient private constructor(
    private val pbClient: PbClient,
    private val addressResolver: ContractAddressResolver,
    private val objectMapper: ObjectMapper,
    private val logger: BilateralContractClientLogger,
) {
    companion object {
        fun builder(pbClient: PbClient, addressResolver: ContractAddressResolver): BilateralContractClientBuilder =
            BilateralContractClientBuilder(pbClient = pbClient, addressResolver = addressResolver)
    }

    class BilateralContractClientBuilder(
        private val pbClient: PbClient,
        private val addressResolver: ContractAddressResolver
    ) {
        private var objectMapper: ObjectMapper? = null
        private var logger: BilateralContractClientLogger? = null

        fun setObjectMapper(objectMapper: ObjectMapper) = apply { this.objectMapper = objectMapper }
        fun setLogger(logger: BilateralContractClientLogger) = apply { this.logger = logger }

        fun build(): BilateralContractClient = BilateralContractClient(
            pbClient = pbClient,
            addressResolver = addressResolver,
            objectMapper = objectMapper ?: ObjectMapperProvider.OBJECT_MAPPER,
            logger = logger ?: BilateralContractClientLogger.Off,
        )
    }

    val contractAddress by lazy { addressResolver.getAddress(pbClient) }

    fun getAsk(id: String): AskOrder = queryContract(
        query = GetAsk(id = id),
    )

    fun getAskOrNull(id: String): AskOrder? = queryContractOrNull(
        query = GetAsk(id = id),
    )

    fun getAskByCollateralId(collateralId: String): AskOrder = queryContract(
        query = GetAskByCollateralId(collateralId = collateralId),
    )

    fun getAskByCollateralIdOrNull(collateralId: String): AskOrder? = queryContractOrNull(
        query = GetAskByCollateralId(collateralId = collateralId),
    )

    fun getBid(id: String): BidOrder = queryContract(
        query = GetBid(id = id),
    )

    fun getBidOrNull(id: String): BidOrder? = queryContractOrNull(
        query = GetBid(id = id),
    )

    fun getMatchReport(askId: String, bidId: String): MatchReport = queryContract(
        query = GetMatchReport(askId = askId, bidId = bidId),
    )

    fun getMatchReportOrNull(askId: String, bidId: String): MatchReport? = queryContractOrNull(
        query = GetMatchReport(askId = askId, bidId = bidId),
    )

    fun searchAsks(searchRequest: ContractSearchRequest): ContractSearchResult<AskOrder> = queryContract(
        query = searchRequest.searchAsks(),
        typeReference = object : TypeReference<ContractSearchResult<AskOrder>>() {},
    )

    fun searchAsksOrNull(searchRequest: ContractSearchRequest): ContractSearchResult<AskOrder>? = queryContractOrNull(
        query = searchRequest.searchAsks(),
        typeReference = object : TypeReference<ContractSearchResult<AskOrder>>() {},
    )

    fun searchBids(searchRequest: ContractSearchRequest): ContractSearchResult<BidOrder> = queryContract(
        query = searchRequest.searchBids(),
        typeReference = object : TypeReference<ContractSearchResult<BidOrder>>() {},
    )

    fun searchBidsOrNull(searchRequest: ContractSearchRequest): ContractSearchResult<BidOrder>? = queryContractOrNull(
        query = searchRequest.searchBids(),
        typeReference = object : TypeReference<ContractSearchResult<BidOrder>>() {},
    )

    fun getContractInfo(): ContractInfo = queryContract(
        query = GetContractInfo(),
    )

    fun getContractInfoOrNull(): ContractInfo? = queryContract(
        query = GetContractInfo(),
    )

    fun createAsk(
        createAsk: CreateAsk,
        signer: Signer,
        options: BilateralBroadcastOptions = BilateralBroadcastOptions(),
    ): CreateAskResponse = executeContract(
        executeMsg = createAsk,
        signer = signer,
        options = options,
        funds = createAsk.getFunds(askFee = this.getContractInfo().askFee),
    ).let { (event, data) ->
        CreateAskResponse(
            askId = event.attribute("ask_id"),
            askOrder = deserializeResponseData(data),
        )
    }

    fun createBid(
        createBid: CreateBid,
        signer: Signer,
        options: BilateralBroadcastOptions = BilateralBroadcastOptions(),
    ): CreateBidResponse = executeContract(
        executeMsg = createBid,
        signer = signer,
        options = options,
        funds = createBid.getFunds(bidFee = this.getContractInfo().bidFee),
    ).let { (event, data) ->
        CreateBidResponse(
            bidId = event.attribute("bid_id"),
            bidOrder = deserializeResponseData(data),
        )
    }

    fun cancelAsk(
        askId: String,
        signer: Signer,
        options: BilateralBroadcastOptions = BilateralBroadcastOptions(),
    ): CancelAskResponse = executeContract(
        executeMsg = CancelAsk(askId),
        signer = signer,
        options = options,
        funds = emptyList(),
    ).let { (event, data) ->
        CancelAskResponse(
            askId = event.attribute("ask_id"),
            cancelledAskOrder = deserializeResponseData(data),
        )
    }

    fun cancelBid(
        bidId: String,
        signer: Signer,
        options: BilateralBroadcastOptions = BilateralBroadcastOptions(),
    ): CancelBidResponse = executeContract(
        executeMsg = CancelBid(bidId),
        signer = signer,
        options = options,
        funds = emptyList(),
    ).let { (event, data) ->
        CancelBidResponse(
            bidId = event.attribute("bid_id"),
            cancelledBidOrder = deserializeResponseData(data),
        )
    }

    // IMPORTANT: The Signer used in this function must be the contract's admin account or the asker associated with the
    // match message's askId.
    fun executeMatch(
        executeMatch: ExecuteMatch,
        signer: Signer,
        options: BilateralBroadcastOptions = BilateralBroadcastOptions(),
    ): ExecuteMatchResponse = executeContract(
        executeMsg = executeMatch,
        signer = signer,
        options = options,
        funds = emptyList(),
    ).let { (event) ->
        ExecuteMatchResponse(
            askId = event.attribute("ask_id"),
            bidId = event.attribute("bid_id"),
            askDeleted = event.attribute("ask_deleted").toBoolean(),
            bidDeleted = event.attribute("bid_deleted").toBoolean(),
        )
    }

    // IMPORTANT: The Signer used in this function must be the contract's admin account.
    fun updateSettings(
        updateSettings: UpdateSettings,
        signer: Signer,
        options: BilateralBroadcastOptions = BilateralBroadcastOptions(),
    ): UpdateSettingsResponse = executeContract(
        executeMsg = updateSettings,
        signer = signer,
        options = options,
        funds = emptyList(),
    ).let { (event) ->
        UpdateSettingsResponse(
            newAdminAddress = event.attributeOrNull("new_admin_address"),
            newAskFee = event.attributeOrNull("new_ask_fee"),
            newBidFee = event.attributeOrNull("new_bid_fee"),
        )
    }

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
        includeLogs: Boolean = true,
    ): MsgExecuteContract {
        val transactionDescription = executeMsg.toLoggingString()
        if (includeLogs) {
            logger.info("START: Generating tx for $transactionDescription")
        }
        return executeMsg.toExecuteMsg(
            objectMapper = objectMapper,
            contractAddress = contractAddress,
            senderBech32Address = senderAddress,
            funds = funds,
        ).also {
            if (includeLogs) {
                logger.info("END: Generating tx for $transactionDescription")
            }
        }
    }

    /**
     * Sends a ContractExecuteMsg to the resolved Metadata Bilateral Exchange contract address with the specified funds.
     * Throws exceptions if the PbClient is misconfigured or if a Provenance Blockchain or smart contract error occurs.
     */
    private fun executeContract(
        executeMsg: ContractExecuteMsg,
        signer: Signer,
        options: BilateralBroadcastOptions,
        funds: List<Coin>,
    ): Pair<Event, String> {
        val transactionDescription = executeMsg.toLoggingString()
        logger.info("START: $transactionDescription")
        val msg = generateProtoExecuteMsg(
            executeMsg = executeMsg,
            senderAddress = signer.address(),
            funds = funds,
            includeLogs = false,
        )
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
        )
            .let { response -> response.singleWasmEvent() to response.txResponse.data }
            .also { logger.info("END: $transactionDescription") }
    }

    /**
     * Dynamic search function allowing for multiple behaviors in search responses and error handling, as configured
     * by each individual call.
     *
     * @param query The query to use as the JSON input to the contract.
     * @param throwExceptions If true, exceptions that occur in the request serialization/deserialization process will
     * be thrown.  If false, exceptions will be logged, but null will be returned.
     * @param typeReference An optional parameter that allows a response that requires complex deserialization with
     * generic type inference to be deserialized properly.  This is not relevant for must queries, but complex response
     * types that include lists of values or other generic parameters need this instead of a class reference for proper
     * deserialization inference.
     */
    private inline fun <T : ContractQueryMsg, reified U : Any> queryContractBase(
        query: T,
        throwExceptions: Boolean,
        typeReference: TypeReference<U>?,
    ): U? = query.toLoggingString().let { queryDescription ->
        try {
            logger.info("START: $queryDescription")
            val responseDataBytes = pbClient.wasmClient.queryWasm(
                QueryOuterClass.QuerySmartContractStateRequest.newBuilder().also { req ->
                    req.address = contractAddress
                    req.queryData = query.toJsonByteString(objectMapper)
                }.build()
            ).data.toByteArray()
            if (typeReference != null) {
                objectMapper.readValue(responseDataBytes, typeReference)
            } else {
                objectMapper.readValue(responseDataBytes, U::class.java)
            }.also {
                logger.info("END: $queryDescription")
            }
        } catch (e: Exception) {
            if (throwExceptions) {
                throw e
            } else {
                logger.error("ERROR: $queryDescription", e)
                null
            }
        }
    }

    private inline fun <T : ContractQueryMsg, reified U : Any> queryContractOrNull(
        query: T,
        typeReference: TypeReference<U>? = null,
    ): U? = queryContractBase(
        query = query,
        throwExceptions = false,
        typeReference = typeReference,
    )

    private inline fun <T : ContractQueryMsg, reified U : Any> queryContract(
        query: T,
        typeReference: TypeReference<U>? = null,
    ): U = queryContractBase(
        query = query,
        throwExceptions = true,
        typeReference = typeReference,
    ) ?: throw NullContractResultException("Got null response from the Metadata Bilateral Exchange contract for query")

    private inline fun <reified T : Any> deserializeResponseData(data: String): T = try {
        // TODO: Cosmos (or maybe CosmWasm) is including some garbage protobuf data in front of the json data after
        // TODO: decoding from hex.  Find a better way to trim off the unneeded data at the ByteArray level versus just
        // TODO: dropping all characters before the opening JSON brace
        val dataString = String(Hex.decode(data))
            .let { dataString -> dataString.drop(dataString.indexOfFirst { it == '{' }) }
        objectMapper.readValue(dataString, T::class.java)
    } catch (e: Exception) {
        throw ProvenanceEventParsingException("Failed to parse [${T::class.qualifiedName}] from response data", e)
    }
}
