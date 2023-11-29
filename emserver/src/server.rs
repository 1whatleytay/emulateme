use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use anyhow::{anyhow, Result};
use prost::Message;
use emulateme::controller::{ControllerFlags, GenericController, NoController};
use emulateme::cpu::Cpu;
use emulateme::interpreter::CpuError;
use emulateme::renderer::{RenderAction, RenderedFrame, Renderer};
use emulateme::rom::Rom;
use emulateme::software::SoftwareRenderer;
use emulateme::state::CpuState;
use crate::delimiter::Delimiter;
use crate::messages::{ActionError, ActionResult, ControllerInput, StreamDetails, EmulatorRequest, FrameContents, FrameDetails, InitializeRequest, InitializeType, Ping, Pong, SetStateResult, StateDetails, StreamRequest};
use crate::messages::stream_request::Contents as StreamContents;
use crate::messages::initialize_request::Contents as InitializeContents;
use crate::messages::emulator_request::Contents as EmulatorContents;

impl From<&ControllerInput> for ControllerFlags {
    fn from(value: &ControllerInput) -> Self {
        let mut flags = ControllerFlags::empty();

        if value.a {
            flags |= ControllerFlags::A;
        }

        if value.b {
            flags |= ControllerFlags::B;
        }

        if value.select {
            flags |= ControllerFlags::SELECT;
        }

        if value.start {
            flags |= ControllerFlags::START;
        }

        if value.up {
            flags |= ControllerFlags::UP;
        }

        if value.down {
            flags |= ControllerFlags::DOWN;
        }

        if value.left {
            flags |= ControllerFlags::LEFT;
        }

        if value.right {
            flags |= ControllerFlags::RIGHT;
        }

        flags
    }
}

type StreamStates = Arc<Mutex<HashMap<u32, StreamDetails>>>;

struct NesInstance<'a> {
    frame: Box<RenderedFrame>,
    renderer: SoftwareRenderer,
    cpu: Cpu<'a, GenericController, NoController>
}

impl<'a> NesInstance<'a> {
    fn get_values(&mut self, requests: &HashMap<String, u32>) -> HashMap<String, u32> {
        let mut values = HashMap::new();

        for (key, address) in requests {
            let address = *address as u16;

            match self.cpu.memory.pass_get(address) {
                Ok(value) => {
                    values.insert(key.clone(), value as u32);
                }
                Err(err) => {
                    println!("Cannot read from memory address {address:04X} \
                                to get key {key} (with error {err})")
                }
            }
        }

        values
    }

    pub fn run_frames(&mut self, skip_frames: usize, input: ControllerFlags) -> Result<(), CpuError> {
        let mut frame_count = 0;

        self.cpu.memory.controllers.0.press(input);

        while frame_count < skip_frames {
            self.cpu.step()?;

            match self.renderer.render(&mut self.cpu.memory.ppu, self.cpu.memory.cycles) {
                RenderAction::None => { },
                RenderAction::SendFrame(frame) => {
                    frame_count += 1;

                    self.cpu.interrupt(self.cpu.vectors.nmi)?;

                    self.frame = frame
                }
            }
        }

        Ok(())
    }

    pub fn new(rom: &Rom) -> NesInstance {
        NesInstance {
            frame: Box::default(),
            cpu: Cpu::new(rom, None, (GenericController::default(), NoController)),
            renderer: SoftwareRenderer::new(),
        }
    }
}

async fn send_message<M: prost::Message>(stream: &mut TcpStream, message: M) -> Result<()> {
    let data = message.encode_to_vec();

    let size = (data.len() as u64).to_be_bytes();

    stream.write_all(&size).await?;
    stream.write_all(&data).await?;

    Ok(())
}

async fn read_into(delimiter: &mut Delimiter, stream: &mut TcpStream) -> Result<()> {
    let mut buffer = [0; 8192];

    let n = match stream.read(&mut buffer).await {
        Ok(n) => n,
        Err(err) => {
            return Err(anyhow!("Connection closed ({err})."))
        }
    };

    if n == 0 {
        return Err(anyhow!("Connection closed (empty read)."))
    }

    delimiter.push(&buffer[0 .. n]);

    Ok(())
}

async fn pong(stream: &mut TcpStream, request: Ping) -> Result<()> {
    send_message(stream, Pong {
        server: "em-server-1".to_string(),
        content: request.content,
    }).await
}

async fn nes_instance(rom: Rom, mut delimiter: Delimiter, mut stream: TcpStream, states: StreamStates) -> Result<()> {
    let mut instance = Box::new(NesInstance::new(&rom));

    loop {
        read_into(&mut delimiter, &mut stream).await?;

        while let Some(packet) = delimiter.pop() {
            let request = match EmulatorRequest::decode(&packet[..]) {
                Ok(n) => n,
                Err(err) => {
                    println!("Failed to decode emulator request ({err})");

                    continue
                }
            };

            let contents = request.contents.ok_or_else(|| anyhow!("Missing contents."))?;

            match contents {
                EmulatorContents::Ping(request) => {
                    pong(&mut stream, request).await?;
                }
                EmulatorContents::GetFrame(frame) => {
                    send_message(&mut stream, FrameDetails {
                        frame: Some(FrameContents {
                            frame: instance.frame.frame.to_vec(),
                            memory_values: instance.get_values(&frame.memory_requests),
                        }),
                    }).await?;
                }
                EmulatorContents::TakeAction(action) => {
                    let flags = action.input.as_ref()
                        .map(ControllerFlags::from)
                        .unwrap_or(ControllerFlags::empty());

                    if let Err(err) = instance.run_frames(action.skip_frames as usize, flags) {
                        send_message(&mut stream, ActionResult {
                            frame: None,
                            error: Some(ActionError {
                                message: format!("CpuError: {err}"),
                            }),
                        }).await?;

                        continue
                    }

                    let memory_values = instance.get_values(&action.memory_requests);

                    if let Some(stream) = action.stream_id {
                        let details = StreamDetails {
                            frame: instance.frame.frame.to_vec(),
                            input: action.input.clone(),
                            memory_values: memory_values.clone(),
                        };

                        let mut states = states.lock().unwrap();

                        states.insert(stream, details);
                    }

                    send_message(&mut stream, ActionResult {
                        frame: Some(FrameContents {
                            frame: instance.frame.frame.to_vec(),
                            memory_values,
                        }),
                        error: None,
                    }).await?;
                }
                EmulatorContents::GetState(_) => {
                    let state: CpuState = (&instance.cpu).into();

                    let bytes = postcard::to_allocvec(&state)
                        .unwrap_or_default();

                    send_message(&mut stream, StateDetails {
                        state: bytes,
                    }).await?;
                }
                EmulatorContents::SetState(state) => {
                    let error = match postcard::from_bytes::<CpuState>(&state.state) {
                        Ok(state) => {
                            let controllers = (GenericController::default(), NoController);

                            if let Some(cpu) = state.restore(&rom, controllers) {
                                instance.cpu = cpu;
                                instance.renderer = SoftwareRenderer::new();

                                None
                            } else {
                                Some("Failed to create CPU instance from state.".to_string())
                            }
                        }
                        Err(err) => Some(format!("{err}"))
                    };

                    send_message(&mut stream, SetStateResult {
                        parse_error: error
                    }).await?;
                }
            }
        }
    }
}

async fn stream_instance(mut delimiter: Delimiter, mut stream: TcpStream, states: StreamStates) -> Result<()> {
    loop {
        read_into(&mut delimiter, &mut stream).await?;

        while let Some(packet) = delimiter.pop() {
            let request = match StreamRequest::decode(&packet[..]) {
                Ok(n) => n,
                Err(err) => {
                    println!("Failed to decode stream request ({err})");

                    continue
                }
            };

            let contents = request.contents.ok_or_else(|| anyhow!("Missing contents."))?;

            match contents {
                StreamContents::Ping(request) => pong(&mut stream, request).await?,
                StreamContents::GetStream(request) => {
                    let frame = {
                        let states = states.lock().unwrap();

                        states.get(&request.stream_id).cloned()
                    };

                    if let Some(frame) = frame {
                        send_message(&mut stream, frame).await?;
                    } else {
                        send_message(&mut stream, StreamDetails {
                            frame: vec![],
                            input: None,
                            memory_values: Default::default(),
                        }).await?;
                    }
                }
            }
        }
    }
}

async fn client_connection(rom: Rom, mut stream: TcpStream, states: StreamStates) -> Result<()> {
    let mut delimiter = Delimiter::default();

    loop {
        read_into(&mut delimiter, &mut stream).await?;

        while let Some(packet) = delimiter.pop() {
            let request = match InitializeRequest::decode(&packet[..]) {
                Ok(n) => n,
                Err(err) => {
                    println!("Failed to decode stream request ({err})");

                    continue
                }
            };

            let contents = request.contents.ok_or_else(|| anyhow!("Missing contents."))?;

            match contents {
                InitializeContents::Ping(request) => pong(&mut stream, request).await?,
                InitializeContents::Initialize(kind) => {
                    let kind = InitializeType::try_from(kind)?;

                    match kind {
                        InitializeType::CreateEmulator => {
                            return nes_instance(rom, delimiter, stream, states).await
                        },
                        InitializeType::OpenStream => {
                            return stream_instance(delimiter, stream, states).await
                        }
                    }
                },
            }
        }
    }
}

pub async fn run_server(rom: &'_ Rom, address: &'_ str) -> Result<()> {
    let stream = TcpListener::bind(address).await?;
    let states: StreamStates = Arc::default();

    println!("Awaiting connections...");

    loop {
        let (stream, _) = stream.accept().await?;

        println!("Connection received!...");

        let rom_clone = rom.clone();
        let states_clone = states.clone();

        tokio::spawn(async move {
            if let Err(error) = client_connection(rom_clone, stream, states_clone).await {
                println!("{error}")
            }
        });
    }
}
