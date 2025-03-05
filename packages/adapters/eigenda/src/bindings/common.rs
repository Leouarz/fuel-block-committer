// This file is @generated by prost-build.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct G1Commitment {
    /// The X coordinate of the KZG commitment. This is the raw byte representation of the field element.
    #[prost(bytes = "vec", tag = "1")]
    pub x: ::prost::alloc::vec::Vec<u8>,
    /// The Y coordinate of the KZG commitment. This is the raw byte representation of the field element.
    #[prost(bytes = "vec", tag = "2")]
    pub y: ::prost::alloc::vec::Vec<u8>,
}
/// BlobCommitment represents commitment of a specific blob, containing its
/// KZG commitment, degree proof, the actual degree, and data length in number of symbols.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlobCommitment {
    #[prost(bytes = "vec", tag = "1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub length_commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub length_proof: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint32, tag = "4")]
    pub length: u32,
}

/// V2 API common

// This file is @generated by prost-build.
/// BlobHeader contains the information describing a blob and the way it is to be dispersed.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlobHeader {
    /// The blob version. Blob versions are pushed onchain by EigenDA governance in an append only fashion and store the
    /// maximum number of operators, number of chunks, and coding rate for a blob. On blob verification, these values
    /// are checked against supplied or default security thresholds to validate the security assumptions of the
    /// blob's availability.
    #[prost(uint32, tag = "1")]
    pub version: u32,
    /// quorum_numbers is the list of quorum numbers that the blob is part of.
    /// Each quorum will store the data, hence adding quorum numbers adds redundancy, making the blob more likely to be retrievable. Each quorum requires separate payment.
    ///
    /// On-demand dispersal is currently limited to using a subset of the following quorums:
    /// - 0: ETH
    /// - 1: EIGEN
    ///
    /// Reserved-bandwidth dispersal is free to use multiple quorums, however those must be reserved ahead of time. The quorum_numbers specified here must be a subset of the ones allowed by the on-chain reservation.
    /// Check the allowed quorum numbers by looking up reservation struct: <https://github.com/Layr-Labs/eigenda/blob/1430d56258b4e814b388e497320fd76354bfb478/contracts/src/interfaces/IPaymentVault.sol#L10>
    #[prost(uint32, repeated, tag = "2")]
    pub quorum_numbers: ::prost::alloc::vec::Vec<u32>,
    /// commitment is the KZG commitment to the blob
    #[prost(message, optional, tag = "3")]
    pub commitment: ::core::option::Option<BlobCommitment>,
    /// payment_header contains payment information for the blob
    #[prost(message, optional, tag = "4")]
    pub payment_header: ::core::option::Option<PaymentHeader>,
}
/// BlobCertificate contains a full description of a blob and how it is dispersed. Part of the certificate
/// is provided by the blob submitter (i.e. the blob header), and part is provided by the disperser (i.e. the relays).
/// Validator nodes eventually sign the blob certificate once they are in custody of the required chunks
/// (note that the signature is indirect; validators sign the hash of a Batch, which contains the blob certificate).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BlobCertificate {
    /// blob_header contains data about the blob.
    #[prost(message, optional, tag = "1")]
    pub blob_header: ::core::option::Option<BlobHeader>,
    /// signature is an ECDSA signature signed by the blob request signer's account ID over the BlobHeader's blobKey,
    /// which is a keccak hash of the serialized BlobHeader, and used to verify against blob dispersal request's account ID
    #[prost(bytes = "vec", tag = "2")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    /// relay_keys is the list of relay keys that are in custody of the blob.
    /// The relays custodying the data are chosen by the Disperser to which the DisperseBlob request was submitted.
    /// It needs to contain at least 1 relay number.
    /// To retrieve a blob from the relay, one can find that relay's URL in the EigenDARelayRegistry contract:
    /// <https://github.com/Layr-Labs/eigenda/blob/master/contracts/src/core/EigenDARelayRegistry.sol>
    #[prost(uint32, repeated, tag = "3")]
    pub relay_keys: ::prost::alloc::vec::Vec<u32>,
}
/// BatchHeader is the header of a batch of blobs
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BatchHeader {
    /// batch_root is the root of the merkle tree of the hashes of blob certificates in the batch
    #[prost(bytes = "vec", tag = "1")]
    pub batch_root: ::prost::alloc::vec::Vec<u8>,
    /// reference_block_number is the block number that the state of the batch is based on for attestation
    #[prost(uint64, tag = "2")]
    pub reference_block_number: u64,
}
/// Batch is a batch of blob certificates
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Batch {
    /// header contains metadata about the batch
    #[prost(message, optional, tag = "1")]
    pub header: ::core::option::Option<BatchHeader>,
    /// blob_certificates is the list of blob certificates in the batch
    #[prost(message, repeated, tag = "2")]
    pub blob_certificates: ::prost::alloc::vec::Vec<BlobCertificate>,
}
/// PaymentHeader contains payment information for a blob.
/// At least one of reservation_period or cumulative_payment must be set, and reservation_period
/// is always considered before cumulative_payment. If reservation_period is set but not valid,
/// the server will reject the request and not proceed with dispersal. If reservation_period is not set
/// and cumulative_payment is set but not valid, the server will reject the request and not proceed with dispersal.
/// Once the server has accepted the payment header, a client cannot cancel or rollback the payment.
/// Every dispersal request will be charged by a multiple of `minNumSymbols` field defined by the payment vault contract.
/// If the request blob size is smaller or not a multiple of `minNumSymbols`, the server will charge the user for the next
/// multiple of `minNumSymbols` (<https://github.com/Layr-Labs/eigenda/blob/1430d56258b4e814b388e497320fd76354bfb478/contracts/src/payments/PaymentVaultStorage.sol#L9>).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PaymentHeader {
    /// The account ID of the disperser client. This account ID is an eth wallet address of the user,
    /// corresponding to the key used by the client to sign the BlobHeader.
    #[prost(string, tag = "1")]
    pub account_id: ::prost::alloc::string::String,
    /// The timestamp should be set as the UNIX timestamp in units of nanoseconds at the time of the dispersal request,
    /// and will be used to determine the reservation period, and compared against the reservation active start and end timestamps
    /// On-chain reservation timestamps are in units of seconds, while the payment header timestamp is in nanoseconds for greater precision.
    /// If the timestamp is not set or is not part of the previous or current reservation period, the request will be rejected.
    /// The reservation period of the dispersal request is used for rate-limiting the user's account against their dedicated
    /// bandwidth. This method requires users to set up reservation accounts with EigenDA team, and the team will set up an
    /// on-chain record of reserved bandwidth for the user for some period of time. The dispersal client's accountant will set
    /// this value to the current timestamp divided by the on-chain configured reservation period interval, mapping each request
    /// to a time-based window and is serialized and parsed as a uint32. The disperser server then validates that it matches
    /// either the current or the previous period.
    ///
    /// Example Usage Flow:
    ///    1. The user sets up a reservation with the EigenDA team, including throughput (symbolsPerSecond), startTimestamp,
    ///       endTimestamp, and reservationPeriodInterval.
    ///    2. When sending a dispersal request at time t, the client fill in the timestamp field with t.
    ///    3. The disperser take timestamp t and checks the reservation period and the user's bandwidth capacity:
    ///        - If the reservation is active (t >= startTimestamp and t < endTimestamp).
    ///        - After rounding up to the nearest multiple of `minNumSymbols` defined by the payment vault contract, the user still has enough bandwidth capacity
    ///          (hasn’t exceeded symbolsPerSecond * reservationPeriodInterval).
    ///        - The request is ratelimited against the current reservation period, and calculated as reservation_period = t / reservationPeriodInterval.
    ///          the request's reservation period must either be the disperser server's current reservation period or the previous reservation period.
    ///    4. Server always go ahead with recording the received request in the current reservation period, and then categorize the scenarios
    ///        - If the remaining bandwidth is sufficient for the request, the dispersal request proceeds.
    ///        - If the remaining bandwidth is not enough for the request, server fills up the current bin and overflowing the extra to a future bin.
    ///        - If the bandwidth has already been exhausted, the request is rejected.
    ///    5. Once the dispersal request signature has been verified, the server will not roll back the payment or the usage records.
    ///       Users should be aware of this when planning their usage. The dispersal client written by EigenDA team takes account of this.
    ///    6. When the reservation ends or usage is exhausted, the client must wait for the next reservation period or switch to on-demand.
    #[prost(int64, tag = "2")]
    pub timestamp: i64,
    /// Cumulative payment is the total amount of tokens paid by the requesting account, including the current request.
    /// This value is serialized as an uint256 and parsed as a big integer, and must match the user’s on-chain deposit limits
    /// as well as the recorded payments for all previous requests. Because it is a cumulative (not incremental) total,
    /// requests can arrive out of order and still unambiguously declare how much of the on-chain deposit can be deducted.
    ///
    /// Example Decision Flow:
    ///    1. In the set up phase, the user must deposit tokens into the EigenDA PaymentVault contract. The payment vault contract
    ///     specifies the minimum number of symbols charged per dispersal, the pricing per symbol, and the maximum global rate for
    ///     on-demand dispersals. The user should calculate the amount of tokens they would like to deposit based on their usage.
    ///     The first time a user make a request, server will immediate read the contract for the on-chain balance. When user runs
    ///     out of on-chain balance, the server will reject the request and not proceed with dispersal. When a user top up on-chain,
    ///     the server will only refresh every few minutes for the top-up to take effect.
    ///    2. The disperser client accounts how many tokens they’ve already paid (previousCumPmt).
    ///    3. They should calculate the payment by rounding up blob size to the nearest multiple of `minNumSymbols` defined by the
    ///     payment vault contract, and calculate the incremental amount of tokens needed for the current request needs based on
    ///     protocol defined pricing.
    ///    4. They take the sum of previousCumPmt + new incremental payment and place it in the “cumulative_payment” field.
    ///    5. The disperser checks this new cumulative total against on-chain deposits and prior records (largest previous payment
    ///     and smallest later payment if exists).
    ///    6. If the payment number is valid, the request is confirmed and disperser proceeds with dispersal; otherwise it’s rejected.
    ///
    #[prost(bytes = "vec", tag = "3")]
    pub cumulative_payment: ::prost::alloc::vec::Vec<u8>,
}
