use std::collections::HashMap;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use anyhow::{anyhow, Result};
use bitflags::bitflags;
use prost::Message;
use emulateme::cpu::Cpu;
use emulateme::interpreter::CpuError;
use emulateme::memory::{Controller, NoController};
use emulateme::renderer::{RenderAction, RenderedFrame, Renderer};
use emulateme::rom::Rom;
use emulateme::software::SoftwareRenderer;
use crate::delimiter::Delimiter;
use crate::messages::{ActionError, ActionResult, ControllerInput, EmulatorDetails, FrameContents, FrameDetails, Pong, Request, Response};
use crate::messages::request::Contents as RequestContents;
use crate::messages::response::Contents as ResponseContents;

#[derive(Default)]
pub struct ControllerFlags(u8);

bitflags! {
    impl ControllerFlags: u8 {
        const A = 0b00000001;
        const B = 0b00000010;
        const SELECT = 0b00000100;
        const START = 0b00001000;
        const UP = 0b00010000;
        const DOWN = 0b00100000;
        const LEFT = 0b01000000;
        const RIGHT = 0b10000000;
    }
}

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

#[derive(Default)]
struct GenericController {
    clock: usize,
    flags: ControllerFlags
}

impl Controller for GenericController {
    fn read(&mut self) -> u8 {
        let clock = self.clock % 8;

        let value = self.flags.0 & (1 << clock) != 0;

        self.clock += 1;

        if value { 1 } else { 0 }
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

        self.cpu.memory.controllers.0.flags = input;

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

    let mut instances: Vec<NesInstance> = vec![];

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

        println!("Received {} bytes from client.", n);

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
                            server: "emserver-1".to_string(),
                            content: request.content,
                        })),
                    }).await?;
                }
                RequestContents::CreateEmulator(_) => {
                    let emulator_id = instances.len() as u64;

                    instances.push(NesInstance::new(&rom));

                    send_message(stream, Response {
                        contents: Some(ResponseContents::EmulatorDetails(EmulatorDetails {
                            emulator_id
                        }))
                    }).await?;
                }
                RequestContents::GetFrame(frame) => {
                    let Some(instance) = instances.get_mut(frame.emulator_id as usize) else {
                        send_message(stream, Response {
                            contents: Some(ResponseContents::FrameDetails(FrameDetails {
                                emulator_id: frame.emulator_id,
                                frame: None,
                            }))
                        }).await?;

                        continue
                    };


                    send_message(stream, Response {
                        contents: Some(ResponseContents::FrameDetails(FrameDetails {
                            emulator_id: frame.emulator_id,
                            frame: Some(FrameContents {
                                frame: instance.frame.frame.to_vec(),
                                memory_values: instance.get_values(&frame.memory_requests),
                            }),
                        }))
                    }).await?;
                }
                RequestContents::TakeAction(action) => {
                    let Some(instance) = instances.get_mut(action.emulator_id as usize) else {
                        send_message(stream, Response {
                            contents: Some(ResponseContents::ActionResult(ActionResult {
                                emulator_id: action.emulator_id,
                                frame: None,
                                error: Some(ActionError {
                                    message: format!("No emulator with id {}", action.emulator_id),
                                }),
                            }))
                        }).await?;

                        continue
                    };

                    let flags = action.input
                        .map(ControllerFlags::from)
                        .unwrap_or(ControllerFlags::empty());

                    if let Err(err) = instance.run_frames(action.skip_frames as usize, flags) {
                        send_message(stream, Response {
                            contents: Some(ResponseContents::ActionResult(ActionResult {
                                emulator_id: action.emulator_id,
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
                            emulator_id: action.emulator_id,
                            frame: Some(FrameContents {
                                frame: instance.frame.frame.to_vec(),
                                memory_values: instance.get_values(&action.memory_requests),
                            }),
                            error: None,
                        }))
                    }).await?;
                }
                _ => { println!("Unknown request contents: {contents:?}") }
            }
        }
    }

    println!("Connection closed.");

    // println!("This is socket thread!");
    //
    // let data = response.encode_to_vec();
    //
    // let size = data.len() as u64;
    //
    // println!("x: {:?}", &size.to_be_bytes());
    //
    // stream.write_all(&size.to_be_bytes()).await.unwrap();
    // stream.write_all(&data).await.unwrap();
    //
    // println!("Sent hello world!");

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
