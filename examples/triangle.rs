#![feature(phase)]

#[phase(plugin)]
extern crate simple_gl_macros;

extern crate gl_init;
extern crate simple_gl;

#[vertex_format]
struct Vertex {
    #[allow(dead_code)]
    iPosition: [f32, ..2],
    #[allow(dead_code)]
    iColor: [f32, ..3],
}

static VERTEX_SRC: &'static str = "
    #version 110

    uniform mat4 uMatrix;

    attribute vec2 iPosition;
    attribute vec3 iColor;

    varying vec3 vColor;

    void main() {
        gl_Position = vec4(iPosition, 0.0, 1.0) * uMatrix;
        vColor = iColor;
    }
";

static FRAGMENT_SRC: &'static str = "
    #version 110
    varying vec3 vColor;

    void main() {
        gl_FragColor = vec4(vColor, 1.0);
    }
";

fn main() {
    use simple_gl::DisplayBuild;

    let display = gl_init::WindowBuilder::new().build_simple_gl().unwrap();

    let program = display.build_program(VERTEX_SRC, FRAGMENT_SRC, None).unwrap();

    let vb = simple_gl::VertexBuffer::new(&display, 
        vec![
            Vertex { iPosition: [-0.5, -0.5], iColor: [0.0, 1.0, 0.0] },
            Vertex { iPosition: [ 0.0,  0.5], iColor: [0.0, 0.0, 1.0] },
            Vertex { iPosition: [ 0.5, -0.5], iColor: [1.0, 0.0, 0.0] },
        ]
    );

    let ib = display.build_index_buffer(simple_gl::TrianglesList,
        &[ 0u16, 1, 2 ]);

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

        for event in display.poll_events().move_iter() {
            match event {
                gl_init::Closed => break 'main,
                _ => ()
            }
        }
    }
}
