// Copyright 2014 The Gfx-rs Developers.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(gl)] pub use self::gl::Device;
#[cfg(gl)] pub use dev = self::gl;
// #[cfg(d3d11)] ... // TODO

use std::comm;
use std::comm::DuplexStream;
use std::kinds::marker;

use GraphicsContext;

#[cfg(gl)] mod gl;
mod shade;

pub type Color = [f32, ..4];
pub type VertexCount = u16;


pub enum Request {
    // Requests that require a reply:
    CallNewBuffer(Vec<f32>),
    CallNewArrayBuffer,
    CallNewShader(char, Vec<u8>),
    CallNewProgram(Vec<dev::Shader>),
    // Requests that don't expect a reply:
    CastClear(Color),
    CastBindProgram(dev::Program),
    CastBindArrayBuffer(dev::ArrayBuffer),
    CastBindAttribute(u8, dev::Buffer, VertexCount, u32, u32),
    CastBindFrameBuffer(dev::FrameBuffer),
    CastDraw(VertexCount, VertexCount),
    CastSwapBuffers,
}

pub enum Reply {
    ReplyNewBuffer(dev::Buffer),
    ReplyNewArrayBuffer(dev::ArrayBuffer),
    ReplyNewShader(dev::Shader),
    ReplyNewProgram(dev::Program),
}

pub struct Client {
    stream: DuplexStream<Request, Reply>,
}

impl Client {
    pub fn clear(&self, color: Color) {
        self.stream.send(CastClear(color));
    }

    pub fn bind_program(&self, prog: dev::Program) {
        self.stream.send(CastBindProgram(prog));
    }

    pub fn bind_array_buffer(&self, abuf: dev::ArrayBuffer) {
        self.stream.send(CastBindArrayBuffer(abuf));
    }

    pub fn bind_attribute(&self, index: u8, buf: dev::Buffer, count: VertexCount, offset: u32, stride: u32) {
        self.stream.send(CastBindAttribute(index, buf, count, offset, stride));
    }

    pub fn bind_frame_buffer(&self, fbo: dev::FrameBuffer) {
        self.stream.send(CastBindFrameBuffer(fbo));
    }

    pub fn draw(&self, offset: VertexCount, count: VertexCount) {
        self.stream.send(CastDraw(offset, count));
    }

    pub fn end_frame(&self) {
        self.stream.send(CastSwapBuffers);
    }

    pub fn new_shader(&self, kind: char, code: Vec<u8>) -> dev::Shader {
        self.stream.send(CallNewShader(kind, code));
        match self.stream.recv() {
            ReplyNewShader(name) => name,
            _ => fail!("unexpected device reply")
        }
    }

    pub fn new_program(&self, shaders: Vec<dev::Shader>) -> dev::Program {
        self.stream.send(CallNewProgram(shaders));
        match self.stream.recv() {
            ReplyNewProgram(name) => name,
            _ => fail!("unexpected device reply")
        }
    }

    pub fn new_buffer(&self, data: Vec<f32>) -> dev::Buffer {
        self.stream.send(CallNewBuffer(data));
        match self.stream.recv() {
            ReplyNewBuffer(name) => name,
            _ => fail!("unexpected device reply")
        }
    }

    pub fn new_array_buffer(&self) -> dev::ArrayBuffer {
        self.stream.send(CallNewArrayBuffer);
        match self.stream.recv() {
            ReplyNewArrayBuffer(name) => name,
            _ => fail!("unexpected device reply")
        }
    }
}

pub struct Server<P> {
    no_send: marker::NoSend,
    no_share: marker::NoShare,
    stream: DuplexStream<Reply, Request>,
    graphics_context: P,
    device: Device,
}

impl<Api, P: GraphicsContext<Api>> Server<P> {
    /// Update the platform. The client must manually update this on the main
    /// thread.
    pub fn update(&mut self) -> bool {
        // Get updates from the renderer and pass on results
        loop {
            match self.stream.try_recv() {
                Ok(CastClear(color)) => {
                    self.device.clear(color.as_slice());
                },
                Ok(CastBindProgram(prog)) => {
                    self.device.bind_program(prog);
                },
                Ok(CastBindArrayBuffer(abuf)) => {
                    self.device.bind_array_buffer(abuf);
                },
                Ok(CastBindAttribute(index, buf, count, offset, stride)) => {
                    self.device.bind_attribute(index, count as u32, offset, stride);
                },
                Ok(CastBindFrameBuffer(fbo)) => {
                    self.device.bind_frame_buffer(fbo);
                },
                Ok(CastDraw(offset, count)) => {
                    self.device.draw(offset as u32, count as u32);
                },
                Ok(CastSwapBuffers) => {
                    break;
                },
                Ok(CallNewBuffer(data)) => {
                    let name = self.device.create_buffer(data.as_slice());
                    self.stream.send(ReplyNewBuffer(name));
                },
                Ok(CallNewArrayBuffer) => {
                    let name = self.device.create_array_buffer();
                    self.stream.send(ReplyNewArrayBuffer(name));
                },
                Ok(CallNewShader(kind, code)) => {
                    let name = self.device.create_shader(kind, code.as_slice());
                    self.stream.send(ReplyNewShader(name));
                },
                Ok(CallNewProgram(code)) => {
                    let name = self.device.create_program(code.as_slice());
                    self.stream.send(ReplyNewProgram(name));
                },
                Err(comm::Empty) => break,
                Err(comm::Disconnected) => return false,
            }
        }
        self.graphics_context.swap_buffers();
        true
    }
}

#[deriving(Show)]
pub enum InitError {}

pub fn init<Api, P: GraphicsContext<Api>>(graphics_context: P, options: super::Options)
        -> Result<(Client, Server<P>), InitError> {
    let (client_stream, server_stream) = comm::duplex();

    let client = Client {
        stream: client_stream,
    };
    let dev = Device::new(options);
    let server = Server {
        no_send: marker::NoSend,
        no_share: marker::NoShare,
        stream: server_stream,
        graphics_context: graphics_context,
        device: dev,
    };

    Ok((client, server))
}
