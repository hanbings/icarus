#![allow(dead_code)]

use crate::raft::action::{AppendEntries, RequestVote};
use crate::raft::client::IrisRaftClient;
use crate::raft::state::{IrisRaftClock, IrisRaftNodeState, IrisRaftNodeType};
use actix_web::web::Data;
use actix_web::Responder;
use futures::stream::FuturesUnordered;
use log::{info, log};
use rand::Rng;
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

/// Receive clock function calls from Iris Client, ideally triggered every 100ms.
///
/// (unverified) Because the concept of random time mechanism already exists in the raft system,
/// the delay caused by the interface call can be ignored.
pub async fn post_check(
    node_state: Data<Mutex<IrisRaftNodeState>>,
    clock: Data<Mutex<IrisRaftClock>>,
    client: Data<Mutex<IrisRaftClient>>,
) -> actix_web::Result<impl Responder> {
    let mut node_state = node_state.lock().unwrap();
    let mut clock = clock.lock().unwrap();
    let mut client = client.lock().unwrap();

    // Update the clock
    clock.clock = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    match node_state.raft_node_type {
        IrisRaftNodeType::Leader => {
            // send heartbeat
            let append_entries = AppendEntries {
                term: node_state.term,
                leader_id: node_state.node.id,
                prev_log_index: node_state.last_applied_index,
                prev_log_term: 0,
                entries: vec![],
                leader_commit_index: node_state.commit_index,
            };

            client.send_heartbeat(append_entries, vec![]);
        }
        IrisRaftNodeType::Candidate => {
            // If the election time exceeds the timeout period tolerated by the cluster,
            // the Candidate should become a new term
            if clock.clock > clock.last_election_time + clock.current_election_timeout_size {
                request_vote(node_state, clock, client).await;

                return Ok(actix_web::web::Json(crate::message::Message::success()));
            }
        }
        IrisRaftNodeType::Follower => {
            // If the heartbeat time exceeds the timeout period tolerated by the cluster,
            // the Leader is offline.
            if clock.clock > clock.last_heartbeat_time + node_state.config.heartbeat_timeout {
                // send request vote
                request_vote(node_state, clock, client).await;

                return Ok(actix_web::web::Json(crate::message::Message::success()));
            }
        }
    }

    Ok(actix_web::web::Json(crate::message::Message::success()))
}

async fn request_vote<'a>(
    mut node_state: MutexGuard<'a, IrisRaftNodeState>,
    mut clock: MutexGuard<'a, IrisRaftClock>,
    mut client: MutexGuard<'a, IrisRaftClient>,
) {
    node_state.raft_node_type = IrisRaftNodeType::Candidate;
    node_state.term += 1;
    node_state.voted_for = Some(node_state.node.id);
    clock.last_election_time = clock.clock;
    clock.current_election_timeout_size = rand::thread_rng()
        .gen_range(node_state.config.election_timeout.0..=node_state.config.election_timeout.1);

    let vote_request = RequestVote {
        node: node_state.node.clone(),
        term: node_state.term,
        candidate_id: node_state.node.id,
        last_log_index: 0,
        last_log_term: 0,
    };

    let mut accepted_node = 0;

    // send request vote
    for node in &node_state.nodes {
        let response = client.vote(vote_request.clone(), node.clone()).await;

        if let Ok(response) = response {
            if response.vote_granted {
                accepted_node += 1;

                if accepted_node > node_state.nodes.len() / 2 {
                    node_state.raft_node_type = IrisRaftNodeType::Leader;

                    info!(
                        "node id:{} become Leader, inner clock time: {}, term: {}",
                        node_state.node.id, clock.clock, node_state.term
                    );

                    let append_entries = AppendEntries {
                        term: node_state.term,
                        leader_id: node_state.node.id,
                        prev_log_index: node_state.last_applied_index,
                        prev_log_term: 0,
                        entries: vec![],
                        leader_commit_index: node_state.commit_index,
                    };

                    client.send_heartbeat(append_entries, node_state.nodes.clone());

                    break;
                }
            }
        }
    }
}
