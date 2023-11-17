use std::collections::HashMap;
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
use crate::messages::{ActionError, ActionResult, ControllerInput, FrameContents, FrameDetails, Pong, Request, Response, SetStateResult, StateDetails};
use crate::messages::request::Contents as RequestContents;
use crate::messages::response::Contents as ResponseContents;

impl From<ControllerInput> for ControllerFlags {
    fn from(value: ControllerInput) -> Self {
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

struct NesInstance<'a> {
    frame: RenderedFrame,
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
            frame: RenderedFrame::default(),
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

async fn client_connection(rom: Rom, stream: &mut TcpStream) -> Result<()> {
    let mut delimiter = Delimiter::default();

    let mut instance = Box::new(NesInstance::new(&rom));

    loop {
        let mut buffer = [0; 8192];

        let n = match stream.read(&mut buffer).await {
            Ok(n) => n,
            Err(err) => {
                println!("Connection closed ({err}).");

                break
            }
        };

        if n == 0 {
            println!("Connection closed (empty read).");

            break
        }

        delimiter.push(&buffer[0 .. n]);

        while let Some(packet) = delimiter.pop() {
            let request = match Request::decode(&packet[..]) {
                Ok(n) => n,
                Err(err) => {
                    println!("Failed to decode response ({err})");

                    continue
                }
            };

            let contents = request.contents.ok_or_else(|| anyhow!("Missing contents."))?;

            match contents {
                RequestContents::Ping(request) => {
                    send_message(stream, Response {
                        contents: Some(ResponseContents::Pong(Pong {
                            server: "em-server-1".to_string(),
                            content: request.content,
                        })),
                    }).await?;
                }
                RequestContents::GetFrame(frame) => {
                    send_message(stream, Response {
                        contents: Some(ResponseContents::FrameDetails(FrameDetails {
                            frame: Some(FrameContents {
                                frame: instance.frame.frame.to_vec(),
                                memory_values: instance.get_values(&frame.memory_requests),
                            }),
                        }))
                    }).await?;
                }
                RequestContents::TakeAction(action) => {
                    let flags = action.input
                        .map(ControllerFlags::from)
                        .unwrap_or(ControllerFlags::empty());

                    if let Err(err) = instance.run_frames(action.skip_frames as usize, flags) {
                        send_message(stream, Response {
                            contents: Some(ResponseContents::ActionResult(ActionResult {
                                frame: None,
                                error: Some(ActionError {
                                    message: format!("CpuError: {err}"),
                                }),
                            }))
                        }).await?;

                        continue
                    }

                    send_message(stream, Response {
                        contents: Some(ResponseContents::ActionResult(ActionResult {
                            frame: Some(FrameContents {
                                frame: instance.frame.frame.to_vec(),
                                memory_values: instance.get_values(&action.memory_requests),
                            }),
                            error: None,
                        }))
                    }).await?;
                }
                RequestContents::GetState(_) => {
                    let state: CpuState = (&instance.cpu).into();

                    let bytes = postcard::to_allocvec(&state)
                        .unwrap_or_default();

                    send_message(stream, Response {
                        contents: Some(ResponseContents::StateDetails(StateDetails {
                            state: bytes,
                        }))
                    }).await?;
                }
                RequestContents::SetState(state) => {
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

                    send_message(stream, Response {
                        contents: Some(ResponseContents::SetStateResult(SetStateResult {
                            parse_error: error
                        }))
                    }).await?;
                }
            }
        }
    }

    println!("Connection closed.");

    Ok(())
}

pub async fn run_server(rom: &'_ Rom, address: &'_ str) -> Result<()> {
    let stream = TcpListener::bind(address).await?;

    println!("Awaiting connections...");

    loop {
        let (mut stream, _) = stream.accept().await?;

        println!("Connection received!...");

        let rom_clone = rom.clone();

        tokio::spawn(async move {
            client_connection(rom_clone, &mut stream).await.unwrap();
        });
    }
}
