use std::time::Instant;

use deimos_render::tfx::expression_vm::{interpreter::InterpreterState, opcodes::Opcode};
use glam::{Vec4, vec4};

fn main() {
    let bytecode: [u8; 0x23] = [
        0x5D, 0x8F, 0x29, 0x00, 0x2A, 0x29, 0x00, 0x42, 0x00, 0x0F, 0x2A, 0x29, 0x00, 0x43, 0x01,
        0x5C, 0xD8, 0xC8, 0xD1, 0x15, 0x29, 0x00, 0x2A, 0x29, 0x00, 0x42, 0x03, 0x0F, 0x2A, 0x29,
        0x00, 0x03, 0x2A, 0x52, 0x01,
    ];

    let mut output = [Vec4::ZERO * 0.0; 2];
    InterpreterState::new(&bytecode)
        .with_debug(true)
        .evaluate(
            &[
                vec4(0.0, 0.0, 1.0, 0.0),
                vec4(1.0, 0.0, 0.0, 0.0),
                vec4(0.0, 0.0, 0.0, 0.0),
                vec4(0.08, 0.92, 0.0, 0.0),
            ],
            &mut output,
        )
        .expect("Failed to execute bytecode");

    println!();
    for (i, out) in output.iter().enumerate() {
        println!("out[{i}]: {out:?}");
    }
}
