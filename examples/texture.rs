#![feature(phase)]

#[phase(plugin)]
extern crate simple_gl_macros;

extern crate gl_init;
extern crate image;
extern crate simple_gl;

#[vertex_format]
struct Vertex {
    #[allow(dead_code)]
    iPosition: [f32, ..2],
    #[allow(dead_code)]
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
    uniform sampler2D uTexture;
    varying vec2 vTexCoords;

    void main() {
        gl_FragColor = texture2D(uTexture, vTexCoords);
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

    let texture = {
        use image::GenericImage;
        use std::io::BufReader;

        static TEXTURE_DATA: &'static [u8] = include_bin!("texture.png");
        let image = match image::load(BufReader::new(TEXTURE_DATA), image::PNG) {
            Ok(img) => img,
            Err(e) => fail!("{}", e)
        };

        let dimensions = image.dimensions();

        let data = image.raw_pixels();
        let data = data.as_slice();
        let data: &[(u8, u8, u8)] = unsafe { std::mem::transmute(data) };
        let data = data.slice_to(data.len() / 3);

        display.build_texture(data, dimensions.val0() as uint,
            dimensions.val1() as uint, 1, 1)
    };

    let mut uniforms = program.build_uniforms();

    uniforms.set_value("uMatrix", [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0f32]
    ]);

    uniforms.set_texture("uTexture", &texture);
    
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
