use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::setting_engine::SettingEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

use crate::bridge::Bridge;
use crate::config::Config;

/// WebSocket signaling message
#[derive(Debug, Serialize, Deserialize)]
struct SignalMessage {
    event: String,
    data: serde_json::Value,
}

/// Handle a new WebSocket connection for WebRTC signaling
#[allow(clippy::too_many_lines)]
pub async fn handle_websocket(socket: WebSocket, config: Arc<Config>, client_id: String) {
    info!(client_id = %client_id, "New WebSocket connection");

    let (ws_sender, ws_receiver) = socket.split();
    let ws_sender = Arc::new(Mutex::new(ws_sender));

    // Create WebRTC peer connection
    let peer = match create_peer_connection(config.public_ip.clone()).await {
        Ok(p) => Arc::new(p),
        Err(e) => {
            error!(client_id = %client_id, error = %e, "Failed to create peer connection");
            return;
        }
    };

    // Create data channels for game data
    // Using ordered mode for reliable file downloads from game server
    let dc_options = RTCDataChannelInit {
        ordered: Some(true),
        ..Default::default()
    };

    // Create "write" channel - for sending data TO the browser
    let write_channel = match peer
        .create_data_channel("write", Some(dc_options.clone()))
        .await
    {
        Ok(dc) => dc,
        Err(e) => {
            error!(client_id = %client_id, error = %e, "Failed to create write channel");
            return;
        }
    };

    // Create "read" channel - for receiving data FROM the browser
    let read_channel = match peer.create_data_channel("read", Some(dc_options)).await {
        Ok(dc) => dc,
        Err(e) => {
            error!(client_id = %client_id, error = %e, "Failed to create read channel");
            return;
        }
    };

    info!(client_id = %client_id, "Created write and read data channels");

    // Setup bridge when both channels are open
    let bridge: Arc<Mutex<Option<Arc<Bridge>>>> = Arc::new(Mutex::new(None));
    let channels_open = Arc::new(std::sync::atomic::AtomicU8::new(0));

    // Track channel opens and start bridge when both are ready
    {
        let config = config.clone();
        let client_id = client_id.clone();
        let bridge = bridge.clone();
        let write_channel_for_bridge = write_channel.clone();
        let read_channel_for_bridge = read_channel.clone();
        let channels_open = channels_open.clone();

        let start_bridge = move |channels_open: Arc<std::sync::atomic::AtomicU8>,
                                 config: Arc<Config>,
                                 client_id: String,
                                 bridge: Arc<Mutex<Option<Arc<Bridge>>>>,
                                 write_channel: Arc<RTCDataChannel>,
                                 read_channel: Arc<RTCDataChannel>| {
            Box::pin(async move {
                let count = channels_open.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                if count == 2 {
                    info!(client_id = %client_id, "Both channels open, starting bridge");

                    match Bridge::new(
                        write_channel,
                        read_channel,
                        &config.server,
                        client_id.clone(),
                    )
                    .await
                    {
                        Ok(b) => {
                            let b = Arc::new(b);
                            *bridge.lock().await = Some(b.clone());
                            tokio::spawn(async move {
                                b.start().await;
                            });
                        }
                        Err(e) => {
                            error!(client_id = %client_id, error = %e, "Failed to create bridge");
                        }
                    }
                }
            })
        };

        // Setup write channel on_open
        let config_clone = config.clone();
        let client_id_clone = client_id.clone();
        let bridge_clone = bridge.clone();
        let write_for_cb = write_channel_for_bridge.clone();
        let read_for_cb = read_channel_for_bridge.clone();
        let channels_open_clone = channels_open.clone();

        write_channel.on_open(Box::new(move || {
            let config = config_clone.clone();
            let client_id = client_id_clone.clone();
            let bridge = bridge_clone.clone();
            let write_channel = write_for_cb.clone();
            let read_channel = read_for_cb.clone();
            let channels_open = channels_open_clone.clone();

            start_bridge(
                channels_open,
                config,
                client_id,
                bridge,
                write_channel,
                read_channel,
            )
        }));

        // Setup read channel on_open
        let config_clone = config.clone();
        let client_id_clone = client_id.clone();
        let bridge_clone = bridge.clone();
        let write_for_cb = write_channel_for_bridge;
        let read_for_cb = read_channel_for_bridge;
        let channels_open_clone = channels_open;

        read_channel.on_open(Box::new(move || {
            let config = config_clone.clone();
            let client_id = client_id_clone.clone();
            let bridge = bridge_clone.clone();
            let write_channel = write_for_cb.clone();
            let read_channel = read_for_cb.clone();
            let channels_open = channels_open_clone.clone();

            start_bridge(
                channels_open,
                config,
                client_id,
                bridge,
                write_channel,
                read_channel,
            )
        }));
    }

    // Send ICE candidates to client
    {
        let ws_sender = ws_sender.clone();
        let client_id = client_id.clone();

        peer.on_ice_candidate(Box::new(move |candidate| {
            let ws_sender = ws_sender.clone();
            let client_id = client_id.clone();

            Box::pin(async move {
                if let Some(c) = candidate {
                    match c.to_json() {
                        Ok(json) => {
                            let msg = SignalMessage {
                                event: "candidate".to_string(),
                                data: serde_json::to_value(json).unwrap_or_default(),
                            };

                            debug!(client_id = %client_id, "Sending ICE candidate");

                            let json_str = serde_json::to_string(&msg).unwrap_or_default();
                            let mut sender = ws_sender.lock().await;
                            if let Err(e) = sender.send(Message::Text(json_str)).await {
                                error!(client_id = %client_id, error = %e, "Failed to send ICE candidate");
                            }
                        }
                        Err(e) => {
                            error!(client_id = %client_id, error = %e, "Failed to serialize ICE candidate");
                        }
                    }
                }
            })
        }));
    }

    // Monitor connection state
    {
        let client_id = client_id.clone();
        let bridge = bridge.clone();

        peer.on_peer_connection_state_change(Box::new(move |state| {
            let client_id = client_id.clone();
            let bridge = bridge.clone();

            Box::pin(async move {
                info!(client_id = %client_id, state = ?state, "Peer connection state changed");

                match state {
                    RTCPeerConnectionState::Failed
                    | RTCPeerConnectionState::Disconnected
                    | RTCPeerConnectionState::Closed => {
                        if let Some(b) = bridge.lock().await.take() {
                            b.shutdown();
                        }
                    }
                    _ => {}
                }
            })
        }));
    }

    // Create and send offer
    let offer = match peer.create_offer(None).await {
        Ok(o) => o,
        Err(e) => {
            error!(client_id = %client_id, error = %e, "Failed to create offer");
            return;
        }
    };

    if let Err(e) = peer.set_local_description(offer.clone()).await {
        error!(client_id = %client_id, error = %e, "Failed to set local description");
        return;
    }

    // Send offer to client
    let offer_msg = SignalMessage {
        event: "offer".to_string(),
        data: serde_json::json!({
            "type": "offer",
            "sdp": offer.sdp
        }),
    };

    {
        let json_str = serde_json::to_string(&offer_msg).unwrap_or_default();
        let mut sender = ws_sender.lock().await;
        if let Err(e) = sender.send(Message::Text(json_str)).await {
            error!(client_id = %client_id, error = %e, "Failed to send offer");
            return;
        }
    }

    info!(client_id = %client_id, "Sent WebRTC offer");

    // Handle incoming WebSocket messages
    handle_ws_messages(ws_receiver, peer, client_id.clone()).await;

    // Cleanup
    if let Some(b) = bridge.lock().await.take() {
        b.shutdown();
    }

    info!(client_id = %client_id, "WebSocket connection closed");
}

/// Handle incoming WebSocket messages (answer, candidates)
async fn handle_ws_messages(
    mut receiver: futures::stream::SplitStream<WebSocket>,
    peer: Arc<RTCPeerConnection>,
    client_id: String,
) {
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let signal: SignalMessage = match serde_json::from_str(&text) {
                    Ok(s) => s,
                    Err(e) => {
                        warn!(client_id = %client_id, error = %e, "Invalid signal message");
                        continue;
                    }
                };

                match signal.event.as_str() {
                    "answer" => {
                        debug!(client_id = %client_id, "Received answer");

                        let sdp = signal
                            .data
                            .get("sdp")
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        let answer = RTCSessionDescription::answer(sdp.to_string()).unwrap();

                        if let Err(e) = peer.set_remote_description(answer).await {
                            error!(client_id = %client_id, error = %e, "Failed to set remote description");
                        }
                    }
                    "candidate" => {
                        debug!(client_id = %client_id, "Received ICE candidate");

                        let candidate: RTCIceCandidateInit = match serde_json::from_value(
                            signal.data,
                        ) {
                            Ok(c) => c,
                            Err(e) => {
                                warn!(client_id = %client_id, error = %e, "Invalid ICE candidate");
                                continue;
                            }
                        };

                        if let Err(e) = peer.add_ice_candidate(candidate).await {
                            error!(client_id = %client_id, error = %e, "Failed to add ICE candidate");
                        }
                    }
                    _ => {
                        warn!(client_id = %client_id, event = %signal.event, "Unknown signal event");
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!(client_id = %client_id, "WebSocket close received");
                break;
            }
            Ok(Message::Ping(_)) => {
                debug!(client_id = %client_id, "Ping received");
                // Pong is handled automatically by axum
            }
            Ok(_) => {}
            Err(e) => {
                error!(client_id = %client_id, error = %e, "WebSocket error");
                break;
            }
        }
    }
}

/// Create a new WebRTC peer connection
async fn create_peer_connection(
    public_ip: Option<String>,
) -> Result<RTCPeerConnection, Box<dyn std::error::Error + Send + Sync>> {
    let mut media_engine = MediaEngine::default();
    media_engine.register_default_codecs()?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)?;

    let mut setting_engine = SettingEngine::default();

    // Set public IP for NAT traversal if provided
    if let Some(ip) = public_ip {
        setting_engine.set_nat_1to1_ips(
            vec![ip],
            webrtc::ice_transport::ice_candidate_type::RTCIceCandidateType::Host,
        );
    }

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_setting_engine(setting_engine)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_string()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer = api.new_peer_connection(config).await?;

    Ok(peer)
}
