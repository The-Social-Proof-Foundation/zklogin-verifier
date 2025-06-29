// Copyright (c) Mysten Labs, Inc.
// Copyright (c) The Social Proof Foundation, LLC.
// SPDX-License-Identifier: Apache-2.0

use axum::response::{IntoResponse, Response};
use axum::{extract::State, Json};
use axum::http::StatusCode;
use fastcrypto::encoding::{Base64, Encoding};
use fastcrypto_zkp::bn254::{
    zk_login::{JwkId, JWK},
    zk_login_api::ZkLoginEnv,
};
use im::hashmap::HashMap as ImHashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use shared_crypto::intent::IntentVersion;
use shared_crypto::intent::{AppId, Intent, IntentMessage, IntentScope, PersonalMessage};
use std::{collections::HashMap, sync::Arc};
use mys_sdk::MysClientBuilder;
use mys_types::committee::EpochId;
use mys_types::{
    base_types::MysAddress,
    crypto::ToFromBytes,
    signature::{AuthenticatorTrait, GenericSignature, VerifyParams},
    transaction::TransactionData,
};
use tracing::info;

#[cfg(test)]
#[path = "test.rs"]
pub mod test;

/// Application state that contains the seed and JWKs.
#[derive(Clone, Debug)]
pub struct AppState {
    /// This is the latest JWKs stored in a mapping from iss -> (kid -> JWK).
    pub jwks: Arc<RwLock<HashMap<JwkId, JWK>>>,
}

/// Request to get salt. It contains the JWT token.
#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyRequest {
    /// The Base64 encoded zkLogin signature.
    pub signature: String,
    /// The Base64 encoded bytes of payload.
    pub bytes: String,
    /// The intent scope, can be either TransactionData or PersonalMessage.
    /// This determines how the `bytes` is deserialized.
    pub intent_scope: IntentScope,
    /// The author of the intent.
    pub author: Option<MysAddress>,
    /// The network to verify the signature against. This determins the
    /// ZkLoginEnv.
    pub network: Option<MysEnv>,
    /// The current epoch to verify the signature against. If not provided,
    /// use `network` to fetch the current epoch.
    pub curr_epoch: Option<EpochId>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub enum MysEnv {
    #[default]
    Mainnet,
    Testnet,
    Devnet,
    Localnet,
}

impl MysEnv {
    fn get_params(&self) -> (&str, ZkLoginEnv) {
        match self {
            MysEnv::Mainnet => ("https://fullnode.mainnet.mysocial.network:8082", ZkLoginEnv::Prod),
            MysEnv::Testnet => ("https://fullnode.testnet.mysocial.network:8082", ZkLoginEnv::Prod),
            MysEnv::Devnet => ("https://fullnode.devnet.mysocial.network:8082", ZkLoginEnv::Test),
            MysEnv::Localnet => ("http://127.0.0.1:9000", ZkLoginEnv::Test),
        }
    }
}

/// Response to get salt.
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    /// The salt value represented as a BigInt
    pub is_verified: bool,
}

/// Error enum for get salt response.
#[derive(Debug, PartialEq)]
pub enum VerifyError {
    /// The Groth16 proof failed to verify.
    GenericError(String),
    /// Fail to parse payload.
    ParsingError,
    /// Error when getting epoch from MySocial client.
    GetEpochError,
}

impl IntoResponse for VerifyError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            VerifyError::GenericError(e) => (StatusCode::BAD_REQUEST, e),
            VerifyError::ParsingError => (StatusCode::BAD_REQUEST, "Parsing error".to_string()),
            VerifyError::GetEpochError => (StatusCode::BAD_REQUEST, "Cannot get epoch".to_string()),
        };
        let body = Json(json!({
            "error": error_message,
        }));
        (status, body).into_response()
    }
}

pub async fn verify(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, VerifyError> {
    info!("verify called");

    let network = payload.network.unwrap_or_default();
    let (url, env) = network.get_params();

    // Use payload.curr_epoch if provided, otherwise fetch current epoch
    // from payload.network (default to Mainnet if not provided).
    let curr_epoch = match payload.curr_epoch {
        Some(curr_epoch) => curr_epoch,
        None => {
            let mys_client = MysClientBuilder::default()
                .build(url)
                .await
                .map_err(|_| VerifyError::GetEpochError)?;

            mys_client
                .governance_api()
                .get_latest_mys_system_state()
                .await
                .map_err(|_| VerifyError::GetEpochError)?
                .epoch
        }
    };
    info!("curr_epoch: {:?}", curr_epoch);

    let parsed: ImHashMap<JwkId, JWK> = state.jwks.read().clone().into_iter().collect();
    let aux_verify_data = VerifyParams::new(parsed, vec![], env, true, true, None);
    info!("aux_verify_data: {:?}", aux_verify_data);

    match GenericSignature::from_bytes(
        &Base64::decode(&payload.signature).map_err(|_| VerifyError::ParsingError)?,
    )
    .map_err(|_| VerifyError::ParsingError)?
    {
        GenericSignature::ZkLoginAuthenticator(zk) => {
            let bytes = Base64::decode(&payload.bytes).map_err(|_| VerifyError::ParsingError)?;
            match payload.intent_scope {
                IntentScope::TransactionData => {
                    let tx_data: TransactionData =
                        bcs::from_bytes(&bytes).map_err(|_| VerifyError::ParsingError)?;
                    let intent_msg = IntentMessage::new(Intent::mys_transaction(), tx_data.clone());
                    let author = tx_data.execution_parts().1;
                    match zk.verify_user_authenticator_epoch(curr_epoch, None) {
                        Ok(_) => Ok(Json(VerifyResponse { is_verified: true })),
                        Err(e) => Err(VerifyError::GenericError(e.to_string())),
                    }
                }
                IntentScope::PersonalMessage => {
                    let tx_data = PersonalMessage { message: bytes };
                    let intent_msg = IntentMessage::new(
                        Intent {
                            scope: IntentScope::PersonalMessage,
                            version: IntentVersion::V0,
                            app_id: AppId::Mys,
                        },
                        tx_data,
                    );
                    let author = match payload.author {
                        Some(author) => author,
                        None => return Err(VerifyError::ParsingError),
                    };
                    match zk.verify_user_authenticator_epoch(curr_epoch, None) {
                        Ok(_) => Ok(Json(VerifyResponse { is_verified: true })),
                        Err(e) => Err(VerifyError::GenericError(e.to_string())),
                    }
                }
                _ => Err(VerifyError::ParsingError),
            }
        }
        _ => Err(VerifyError::ParsingError),
    }
}
