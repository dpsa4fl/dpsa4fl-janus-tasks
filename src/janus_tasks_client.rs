use std::time::UNIX_EPOCH;

use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine};
use fixed::types::extra::U31;
use fixed::FixedI32;
use http::StatusCode;
use janus_collector::{Collection, Collector, CollectorParameters};
use janus_core::{
    hpke::{generate_hpke_config_and_private_key, HpkeKeypair},
    task::AuthenticationToken,
};
use janus_messages::{
    query_type::TimeInterval, Duration, HpkeAeadId, HpkeKdfId, HpkeKemId, Interval, Query, Role,
    TaskId, Time,
};
use prio::flp::types::fixedpoint_l2::zero_privacy_parameter;
use prio::{codec::Encode, vdaf::prio3::Prio3Aes128FixedPointBoundedL2VecSum};
use rand::random;

use janus_aggregator::task::PRIO3_AES128_VERIFY_KEY_LENGTH;
use reqwest::Url;

use crate::core::{GetVdafParameterRequest, GetVdafParameterResponse, VdafParameter, MainLocations, TasksLocations};

use super::core::{
    CreateTrainingSessionRequest, CreateTrainingSessionResponse, Locations, StartRoundRequest,
    TrainingSessionId,
};

pub type Fx = FixedI32<U31>;
pub const TIME_PRECISION: u64 = 3600;

pub struct JanusTasksClient
{
    http_client: reqwest::Client,
    location: Locations,
    hpke_keypair: HpkeKeypair,
    leader_auth_token: AuthenticationToken,
    collector_auth_token: AuthenticationToken,
    vdaf_parameter: VdafParameter,
}

impl JanusTasksClient
{
    pub fn new(location: Locations, vdaf_parameter: VdafParameter) -> Self
    {
        let leader_auth_token = rand::random::<[u8; 16]>().to_vec().into();
        let collector_auth_token = rand::random::<[u8; 16]>().to_vec().into();

        let hpke_id = random::<u8>().into();
        let hpke_keypair = generate_hpke_config_and_private_key(
            hpke_id,
            // These algorithms should be broadly compatible with other DAP implementations, since they
            // are required by section 6 of draft-ietf-ppm-dap-02.
            HpkeKemId::X25519HkdfSha256,
            HpkeKdfId::HkdfSha256,
            HpkeAeadId::Aes128Gcm,
        );
        JanusTasksClient {
            http_client: reqwest::Client::new(),
            location,
            hpke_keypair,
            leader_auth_token,
            collector_auth_token,
            vdaf_parameter,
        }
    }

    pub async fn create_session(&self) -> Result<TrainingSessionId>
    {
        let leader_auth_token_encoded =
            general_purpose::URL_SAFE_NO_PAD.encode(self.leader_auth_token.as_bytes());
        let collector_auth_token_encoded =
            general_purpose::URL_SAFE_NO_PAD.encode(self.collector_auth_token.as_bytes());
        let verify_key = rand::random::<[u8; PRIO3_AES128_VERIFY_KEY_LENGTH]>();
        let verify_key_encoded = general_purpose::URL_SAFE_NO_PAD.encode(&verify_key);

        let make_request = |role, id| CreateTrainingSessionRequest {
            training_session_id: id,
            role,
            verify_key_encoded: verify_key_encoded.clone(),
            collector_hpke_config: self.hpke_keypair.config().clone(),
            collector_auth_token_encoded: collector_auth_token_encoded.clone(),
            leader_auth_token_encoded: leader_auth_token_encoded.clone(),
            vdaf_parameter: self.vdaf_parameter.clone(),
        };

        // send request to leader first
        // and get response
        let leader_response = self
            .http_client
            .post(
                self.location
                    .tasks.external_leader
                    .join("/create_session")
                    .unwrap(),
            )
            .json(&make_request(Role::Leader, None))
            .send()
            .await?;
        let leader_response = match leader_response.status()
        {
            StatusCode::OK =>
            {
                let response: CreateTrainingSessionResponse = leader_response.json().await?;
                response
            }
            res =>
            {
                return Err(anyhow!("Got error from leader: {res}"));
            }
        };

        let helper_response = self
            .http_client
            .post(
                self.location
                    .tasks.external_helper
                    .join("/create_session")
                    .unwrap(),
            )
            .json(&make_request(
                Role::Helper,
                Some(leader_response.training_session_id),
            ))
            .send()
            .await?;

        let helper_response = match helper_response.status()
        {
            StatusCode::OK =>
            {
                let response: CreateTrainingSessionResponse = helper_response.json().await?;
                response
            }
            res =>
            {
                return Err(anyhow!("Got error from helper: {res}"));
            }
        };

        assert!(
            helper_response.training_session_id == leader_response.training_session_id,
            "leader and helper have different training session id!"
        );

        Ok(leader_response.training_session_id)
    }

    /// Send requests to the aggregators to end a new round and delete the associated data.
    pub async fn end_session(&self, training_session_id: TrainingSessionId) -> Result<()>
    {
        let leader_response = self
            .http_client
            .post(
                self.location
                    .tasks.external_leader
                    .join("/end_session")
                    .unwrap(),
            )
            .json(&training_session_id)
            .send()
            .await?;

        let helper_response = self
            .http_client
            .post(
                self.location
                    .tasks.external_helper
                    .join("/end_session")
                    .unwrap(),
            )
            .json(&training_session_id)
            .send()
            .await?;

        match (leader_response.status(), helper_response.status())
        {
            (StatusCode::OK, StatusCode::OK) => Ok(()),
            (res1, res2) => Err(anyhow!(
                "Ending session not successful, results are: \n{res1}\n\n{res2}"
            )),
        }
    }

    /// Send requests to the aggregators to start a new round.
    ///
    /// We return the task id with which the task can be collected.
    pub async fn start_round(&self, training_session_id: TrainingSessionId) -> Result<TaskId>
    {
        let task_id: TaskId = random();
        let task_id_encoded = general_purpose::URL_SAFE_NO_PAD.encode(&task_id.get_encoded());
        let request: StartRoundRequest = StartRoundRequest {
            training_session_id,
            task_id_encoded,
        };
        let leader_response = self
            .http_client
            .post(
                self.location
                    .tasks.external_leader
                    .join("/start_round")
                    .unwrap(),
            )
            .json(&request)
            .send()
            .await?;

        let helper_response = self
            .http_client
            .post(
                self.location
                    .tasks.external_helper
                    .join("/start_round")
                    .unwrap(),
            )
            .json(&request)
            .send()
            .await?;

        match (leader_response.status(), helper_response.status())
        {
            (StatusCode::OK, StatusCode::OK) => Ok(task_id),
            (res1, res2) => Err(anyhow!(
                "Starting round not successful, results are: \n{res1}\n\n{res2}"
            )),
        }
    }

    /// Collect results
    pub async fn collect(&self, task_id: TaskId) -> Result<Collection<Vec<f64>, TimeInterval>>
    {
        let params = CollectorParameters::new(
            task_id,
            self.location.main.external_leader.clone(),
            self.collector_auth_token.clone(),
            self.hpke_keypair.config().clone(),
            self.hpke_keypair.private_key().clone(),
        );

        let vdaf_collector =
            Prio3Aes128FixedPointBoundedL2VecSum::<Fx>::new_aes128_fixedpoint_boundedl2_vec_sum(
                2,
                self.vdaf_parameter.gradient_len,
                zero_privacy_parameter(),
            )?;

        let collector_http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        let collector_client = Collector::new(params, vdaf_collector, collector_http_client);

        let start = UNIX_EPOCH.elapsed()?.as_secs();
        let rounded_start = (start / TIME_PRECISION) * TIME_PRECISION;
        let real_start = Time::from_seconds_since_epoch(rounded_start - TIME_PRECISION * 5);
        let duration = Duration::from_seconds(TIME_PRECISION * 15);

        let aggregation_parameter = ();

        let host = self
            .location
            .main.external_leader
            .host()
            .ok_or(anyhow!("Couldnt get hostname"))?;
        let port = self
            .location
            .main.external_leader
            .port()
            .ok_or(anyhow!("Couldnt get port"))?;

        println!("patched host and port are: {host} -:- {port}");

        println!("collecting result now");

        let result = collector_client
            .collect_with_rewritten_url(
                Query::new(Interval::new(real_start, duration)?),
                &aggregation_parameter,
                &host.to_string(),
                port,
            )
            .await?;

        Ok(result)
    }
}

//////////////////////////////////////////////////////
// client functionality for dpas4fl clients

pub async fn get_vdaf_parameter_from_task(
    tasks_server: Url,
    task_id: TaskId,
) -> Result<VdafParameter>
{
    let task_id_encoded = general_purpose::URL_SAFE_NO_PAD.encode(&task_id.get_encoded());

    let request = GetVdafParameterRequest { task_id_encoded };

    let response = reqwest::Client::new()
        .post(tasks_server.join("/get_vdaf_parameter").unwrap())
        .json(&request)
        .send()
        .await?;

    let param: Result<GetVdafParameterResponse> =
        response.json().await.map_err(|e| anyhow!("got err: {e}"));

    param.map(|x| x.vdaf_parameter)
}


pub async fn get_main_locations(
    tasks_servers: TasksLocations,
) -> Result<MainLocations>
{
    let response_leader = reqwest::Client::new()
        .get(tasks_servers.external_leader.join("/get_main_locations").unwrap())
        .send()
        .await?;

    let response_helper = reqwest::Client::new()
        .get(tasks_servers.external_helper.join("/get_main_locations").unwrap())
        .send()
        .await?;

    let result_leader : Result<MainLocations, _> = response_leader.json().await;
    let result_helper : Result<MainLocations, _> = response_helper.json().await;

    match (result_leader, result_helper)
    {
        (Ok(a), Ok(b)) => {
            if a == b
            {
                Ok(a)
            }
            else
            {
                Err(anyhow!("The aggregators returned different main locations ({a:?} and {b:?})"))
            }
        },
        (res1, res2) => Err(anyhow!(
            "Getting main locations not successful, results are: \n{res1:?}\n\n{res2:?}"
        )),
    }
}
