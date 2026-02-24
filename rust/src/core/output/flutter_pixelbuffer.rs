use std::{
    sync::{Arc, Weak},
    thread,
    time::Duration,
};

use anyhow::Result;
use flume::{Receiver, Selector, Sender};
use irondash_texture::Texture;
use log::{debug, info, warn};

use crate::{
    core::{
        input::{InputCommand, InputCommandSender, InputEvent, InputEventReceiver},
        texture::{
            flutter::{SharedSendableTexture, TextureSession},
            payload::PayloadHolder,
            FlutterTextureSession,
        },
        types::DartStateStream,
    },
    utils::{invoke_on_platform_main_thread, LogErr},
};

#[derive(Debug, Clone)]
pub enum OutputCommand {
    Terminate,
    Seek { ts: i64 },
}

#[derive(Clone)]
pub struct FlutterPixelBufferHandle {
    command_tx: Sender<OutputCommand>,
}

impl FlutterPixelBufferHandle {
    pub fn send(&self, command: OutputCommand) -> Result<()> {
        self.command_tx
            .send(command)
            .map_err(|err| anyhow::anyhow!("pixelbuffer command send failed: {}", err))
    }

    pub fn sender(&self) -> Sender<OutputCommand> {
        self.command_tx.clone()
    }
}

pub fn create_flutter_pixelbuffer(
    session_id: i64,
    engine_handle: i64,
    state_sink: DartStateStream,
    input_event_rx: InputEventReceiver,
    input_command_tx: InputCommandSender,
) -> Result<(FlutterPixelBufferHandle, Weak<PayloadHolder>, i64)> {
    let payload_holder = Arc::new(PayloadHolder::new());
    let payload_holder_weak = Arc::downgrade(&payload_holder);
    let payload_holder_for_texture = Arc::clone(&payload_holder);

    let (sendable_texture, texture_id) = invoke_on_platform_main_thread(move || -> Result<_> {
        let texture = Texture::new_with_provider(engine_handle, payload_holder_for_texture)?;
        let texture_id = texture.id();
        Ok((texture.into_sendable_texture(), texture_id))
    })?;

    let texture_session = Arc::new(TextureSession::new(
        texture_id,
        Arc::downgrade(&sendable_texture),
        payload_holder_weak.clone(),
    ));
    let texture_session: Arc<dyn FlutterTextureSession> = texture_session;

    let (command_tx, command_rx) = flume::unbounded();
    let handle = FlutterPixelBufferHandle {
        command_tx: command_tx.clone(),
    };

    thread::spawn(move || {
        run_output_loop(
            session_id,
            texture_session,
            sendable_texture,
            state_sink,
            input_event_rx,
            command_rx,
            input_command_tx,
        );
    });

    Ok((handle, payload_holder_weak, texture_id))
}

fn run_output_loop(
    session_id: i64,
    texture_session: Arc<dyn FlutterTextureSession>,
    sendable_texture: SharedSendableTexture,
    state_sink: DartStateStream,
    input_event_rx: Receiver<InputEvent>,
    command_rx: Receiver<OutputCommand>,
    input_command_tx: InputCommandSender,
) {
    let mut commands_closed = false;
    let mut events_closed = false;
    loop {
        if commands_closed && events_closed {
            break;
        }
        if commands_closed {
            match input_event_rx.recv() {
                Ok(event) => handle_event(event, &texture_session, &state_sink),
                Err(_) => events_closed = true,
            }
            continue;
        }
        if events_closed {
            match command_rx.recv() {
                Ok(command) => {
                    if handle_command(session_id, command, &input_command_tx, &texture_session) {
                        finalize_texture(session_id, sendable_texture);
                        return;
                    }
                }
                Err(_) => commands_closed = true,
            }
            continue;
        }
        enum Selection {
            Command(Result<OutputCommand, flume::RecvError>),
            Event(Result<InputEvent, flume::RecvError>),
        }

        let selection = Selector::new()
            .recv(&command_rx, |msg| Selection::Command(msg))
            .recv(&input_event_rx, |msg| Selection::Event(msg))
            .wait();
        match selection {
            Selection::Command(Ok(command)) => {
                if handle_command(session_id, command, &input_command_tx, &texture_session) {
                    finalize_texture(session_id, sendable_texture);
                    return;
                }
            }
            Selection::Command(Err(_)) => commands_closed = true,
            Selection::Event(Ok(event)) => handle_event(event, &texture_session, &state_sink),
            Selection::Event(Err(_)) => events_closed = true,
        }
    }

    finalize_texture(session_id, sendable_texture);
}

fn handle_command(
    _session_id: i64,
    command: OutputCommand,
    input_command_tx: &InputCommandSender,
    texture_session: &Arc<dyn FlutterTextureSession>,
) -> bool {
    match command {
        OutputCommand::Terminate => {
            let _ = input_command_tx.send(InputCommand::Terminate);
            texture_session.terminate();
            true
        }
        OutputCommand::Seek { ts } => {
            let _ = input_command_tx.send(InputCommand::Seek { ts });
            false
        }
    }
}

fn handle_event(
    event: InputEvent,
    texture_session: &Arc<dyn FlutterTextureSession>,
    state_sink: &DartStateStream,
) {
    match event {
        InputEvent::FrameAvailable => texture_session.mark_frame_available(),
        InputEvent::State(state) => {
            state_sink.add(state).log_err();
        }
    }
}

fn finalize_texture(session_id: i64, sendable_texture: SharedSendableTexture) {
    let mut retry_count = 0;
    const MAX_RETRIES: usize = 50;
    while retry_count < MAX_RETRIES {
        let strong_count = Arc::strong_count(&sendable_texture);
        debug!(
            "Session {} texture strong_count={} attempt={}",
            session_id, strong_count, retry_count
        );
        if strong_count == 1 {
            break;
        }
        debug!(
            "Waiting for texture references to be dropped for session id: {}. attempt({})",
            session_id, retry_count
        );
        thread::sleep(Duration::from_millis(500));
        retry_count += 1;
    }
    if retry_count == MAX_RETRIES {
        warn!(
            "Forcefully dropping texture for session id: {}, texture still held elsewhere.",
            session_id
        );
    }
    invoke_on_platform_main_thread(move || {
        drop(sendable_texture);
        info!("Destroyed stream session for session id: {}", session_id);
    });
}
