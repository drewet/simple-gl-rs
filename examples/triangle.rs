#![feature(phase)]

#[phase(plugin)]
extern crate simple_gl_macros;

extern crate gl_init;
extern crate simple_gl;

#[vertex_format]
struct Vertex {
    iPosition: [f32, ..2],
    iTexCoords: [f32, ..2],
}

static VERTEX_SRC: &'static str = "
    #version 110

    uniform mat4 uMatrix;

    attribute vec2 iPosition;
    attribute vec2 iTexCoords;

    varying vec2 vTexCoords;

    void main() {
        gl_Position = vec4(iPosition, 0.0, 1.0) * uMatrix;
        vTexCoords = iTexCoords;
    }
";

static FRAGMENT_SRC: &'static str = "
    #version 110
    varying vec2 vTexCoords;

    void main() {
        gl_FragColor = vec4(vTexCoords.x, vTexCoords.y, 0.0, 1.0);
    }
";

fn main() {
    use simple_gl::DisplayBuild;

    let display = gl_init::WindowBuilder::new().build_simple_gl().unwrap();

    let program = display.build_program(VERTEX_SRC, FRAGMENT_SRC).unwrap();

    let vb = display.build_vertex_buffer(
        vec![
            Vertex { iPosition: [-1.0, -1.0], iTexCoords: [0.0, 1.0] },
            Vertex { iPosition: [-1.0,  1.0], iTexCoords: [0.0, 0.0] },
            Vertex { iPosition: [ 1.0,  1.0], iTexCoords: [1.0, 0.0] },
            Vertex { iPosition: [ 1.0, -1.0], iTexCoords: [1.0, 1.0] }
        ]
    );

    let ib = display.build_index_buffer(simple_gl::TrianglesList,
        &[ 0 as u16, 1, 2, 0, 2, 3 ]);

    let mut uniforms = program.build_uniforms();

    uniforms.set_value("uMatrix", [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0f32]
    ]);
    
    'main: loop {
        use std::io::timer;
        use std::time::Duration;

        display.draw(&vb, &ib, &uniforms);
        display.end_frame();
        timer::sleep(Duration::milliseconds(17));

        for event in display.recv().move_iter() {
            match event {
                gl_init::Closed => break 'main,
                _ => ()
            }
        }
    }
}
