//! Handles the encoding and decoding of pubsub messages.

use crate::topics::{GossipEncoding, GossipKind, GossipTopic};
use crate::SubnetId;
use crate::{Topic, TopicHash};
use ssz::{Decode, Encode};
use std::boxed::Box;
use types::{
    AggregateAndProof, Attestation, AttesterSlashing, BeaconBlock, EthSpec, ProposerSlashing,
    VoluntaryExit,
};

/// Messages that are passed to and from the pubsub (Gossipsub) behaviour. These are encoded and
/// decoded upstream.
#[derive(Debug, Clone, PartialEq)]
pub enum PubsubMessage<T: EthSpec> {
    /// Gossipsub message providing notification of a new block.
    BeaconBlock(Box<BeaconBlock<T>>),
    /// Gossipsub message providing notification of a Aggregate attestation and associated proof.
    AggregateAndProofAttestation(Box<AggregateAndProof<T>>),
    /// Gossipsub message providing notification of a raw un-aggregated attestation with its shard id.
    Attestation(Box<(SubnetId, Attestation<T>)>),
    /// Gossipsub message providing notification of a voluntary exit.
    VoluntaryExit(Box<VoluntaryExit>),
    /// Gossipsub message providing notification of a new proposer slashing.
    ProposerSlashing(Box<ProposerSlashing>),
    /// Gossipsub message providing notification of a new attester slashing.
    AttesterSlashing(Box<AttesterSlashing<T>>),
}

impl<T: EthSpec> PubsubMessage<T> {
    /* Note: This is assuming we are not hashing topics. If we choose to hash topics, these will
     * need to be modified.
     *
     * Also note that a message can be associated with many topics. As soon as one of the topics is
     * known we match. If none of the topics are known we return an unknown state.
     */
    pub fn decode(topics: &[TopicHash], data: &[u8]) -> Result<Self, String> {
        let mut unknown_topics = Vec::new();
        for topic in topics {
            match GossipTopic::decode(topic.as_str()) {
                Err(_) => {
                    unknown_topics.push(topic);
                    continue;
                }
                Ok(gossip_topic) => {
                    match gossip_topic.encoding() {
                        // group each part by encoding type
                        GossipEncoding::SSZ => {
                            // the ssz decoders
                            match gossip_topic.kind() {
                                GossipKind::BeaconAggregateAndProof => {
                                    let agg_and_proof = AggregateAndProof::from_ssz_bytes(data)
                                        .map_err(|e| format!("{:?}", e))?;
                                    return Ok(PubsubMessage::AggregateAndProofAttestation(
                                        Box::new(agg_and_proof),
                                    ));
                                }
                                GossipKind::CommitteeIndex(subnet_id) => {
                                    let attestation = Attestation::from_ssz_bytes(data)
                                        .map_err(|e| format!("{:?}", e))?;
                                    return Ok(PubsubMessage::Attestation(Box::new((
                                        subnet_id,
                                        attestation,
                                    ))));
                                }
                                GossipKind::BeaconBlock => {
                                    let beacon_block = BeaconBlock::from_ssz_bytes(data)
                                        .map_err(|e| format!("{:?}", e))?;
                                    return Ok(PubsubMessage::BeaconBlock(Box::new(beacon_block)));
                                }
                                GossipKind::VoluntaryExit => {
                                    let voluntary_exit = VoluntaryExit::from_ssz_bytes(data)
                                        .map_err(|e| format!("{:?}", e))?;
                                    return Ok(PubsubMessage::VoluntaryExit(Box::new(
                                        voluntary_exit,
                                    )));
                                }
                                GossipKind::ProposerSlashing => {
                                    let proposer_slashing = ProposerSlashing::from_ssz_bytes(data)
                                        .map_err(|e| format!("{:?}", e))?;
                                    return Ok(PubsubMessage::ProposerSlashing(Box::new(
                                        proposer_slashing,
                                    )));
                                }
                                GossipKind::AttesterSlashing => {
                                    let attester_slashing = AttesterSlashing::from_ssz_bytes(data)
                                        .map_err(|e| format!("{:?}", e))?;
                                    return Ok(PubsubMessage::AttesterSlashing(Box::new(
                                        attester_slashing,
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(format!("Unknown gossipsub topics: {:?}", unknown_topics))
    }

    /// Encodes a pubsub message based on the topic encodings. The first known encoding is used. If
    /// no encoding is known, and error is returned.
    pub fn encode(&self, encoding: &GossipEncoding) -> Vec<u8> {
        match encoding {
            GossipEncoding::SSZ => {
                // SSZ Encodings
                let bytes = match self {
                    PubsubMessage::BeaconBlock(data) => data.as_ssz_bytes()
                    | PubsubMessage::VoluntaryExit(data)
                    | PubsubMessage::ProposerSlashing(data)
                    | PubsubMessage::AttesterSlashing(data)
                    | PubsubMessage::Unknown(data) => data.as_ssz_bytes(),

                    PubsubMessage::Attestation(other) => Vec::new(),
                };
                return bytes;
            }
        }
    }
}
