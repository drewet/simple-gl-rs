#![feature(phase)]

#[phase(plugin)]
extern crate simple_gl_macros;

extern crate gl_init;
extern crate simple_gl;

#[vertex_format]
struct ToTextureVertex {
    #[allow(dead_code)]
    iPosition: [f32, ..2],
    #[allow(dead_code)]
    iColor: [f32, ..3],
}

static TO_TEXTURE_VERTEX_SRC: &'static str = "
    #version 110

    attribute vec2 iPosition;
    attribute vec3 iColor;

    varying vec3 vColor;

    void main() {
        gl_Position = vec4(iPosition, 0.0, 1.0);
        vColor = iColor;
    }
";

static TO_TEXTURE_FRAGMENT_SRC: &'static str = "
    #version 110
    varying vec3 vColor;

    void main() {
        gl_FragColor = vec4(vColor, 1.0);
    }
";

#[vertex_format]
struct ToDestVertex {
    #[allow(dead_code)]
    iPosition: [f32, ..2],
    #[allow(dead_code)]
    iTexCoords: [f32, ..2],
}

static TO_DEST_VERTEX_SRC: &'static str = "
    #version 110

    attribute vec2 iPosition;
    attribute vec2 iTexCoords;

    varying vec2 vTexCoords;

    void main() {
        gl_Position = vec4(iPosition, 0.0, 1.0);
        vTexCoords = iTexCoords;
    }
";

static TO_DEST_FRAGMENT_SRC: &'static str = "
    #version 110
    uniform sampler2D uTexture;
    varying vec2 vTexCoords;

    const float blurSize = 4.0 / 512.0;

    void main() {
        vec4 sum = vec4(0.0);

        sum += texture2D(uTexture, vec2(vTexCoords.x - 4.0 * blurSize, vTexCoords.y)) * 0.05;
        sum += texture2D(uTexture, vec2(vTexCoords.x - 3.0 * blurSize, vTexCoords.y)) * 0.09;
        sum += texture2D(uTexture, vec2(vTexCoords.x - 2.0 * blurSize, vTexCoords.y)) * 0.12;
        sum += texture2D(uTexture, vec2(vTexCoords.x - blurSize, vTexCoords.y)) * 0.15;
        sum += texture2D(uTexture, vec2(vTexCoords.x, vTexCoords.y)) * 0.16;
        sum += texture2D(uTexture, vec2(vTexCoords.x + blurSize, vTexCoords.y)) * 0.15;
        sum += texture2D(uTexture, vec2(vTexCoords.x + 2.0 * blurSize, vTexCoords.y)) * 0.12;
        sum += texture2D(uTexture, vec2(vTexCoords.x + 3.0 * blurSize, vTexCoords.y)) * 0.09;
        sum += texture2D(uTexture, vec2(vTexCoords.x + 4.0 * blurSize, vTexCoords.y)) * 0.05;

        gl_FragColor = sum;
    }
";

fn main() {
    use simple_gl::DisplayBuild;

    let display = gl_init::WindowBuilder::new().build_simple_gl().unwrap();

    let to_texture_program = display.build_program(TO_TEXTURE_VERTEX_SRC, TO_TEXTURE_FRAGMENT_SRC, None).unwrap();
    let to_dest_program = display.build_program(TO_DEST_VERTEX_SRC, TO_DEST_FRAGMENT_SRC, None).unwrap();

    let to_texture_vertex_buffer = simple_gl::VertexBuffer::new(&display, 
        vec![
            ToTextureVertex { iPosition: [-0.5, -0.5], iColor: [0.0, 1.0, 0.0] },
            ToTextureVertex { iPosition: [ 0.0,  0.5], iColor: [0.0, 0.0, 1.0] },
            ToTextureVertex { iPosition: [ 0.5, -0.5], iColor: [1.0, 0.0, 0.0] },
        ]
    );

    let to_texture_index_buffer = display.build_index_buffer(simple_gl::TrianglesList,
        &[ 0u16, 1, 2 ]);

    let to_dest_vertex_buffer = simple_gl::VertexBuffer::new(&display, 
        vec![
            ToDestVertex { iPosition: [-1.0,  1.0], iTexCoords: [0.0, 1.0] },
            ToDestVertex { iPosition: [ 1.0,  1.0], iTexCoords: [1.0, 1.0] },
            ToDestVertex { iPosition: [-1.0, -1.0], iTexCoords: [0.0, 0.0] },
            ToDestVertex { iPosition: [ 1.0, -1.0], iTexCoords: [1.0, 0.0] },
        ]
    );

    let to_dest_index_buffer = display.build_index_buffer(simple_gl::TrianglesList,
        &[ 0u16, 1, 2, 1, 3, 2 ]);

    let to_texture_uniforms = to_texture_program.build_uniforms();
    let mut to_dest_uniforms = to_dest_program.build_uniforms();

    let mut texture = display.build_texture(Vec::from_elem(1024 * 768, 0u8).as_slice(), 1024, 768, 1, 1);
    
    'main: loop {
        use std::io::timer;
        use std::time::Duration;

        texture.draw().draw(&to_texture_vertex_buffer, &to_texture_index_buffer, &to_texture_uniforms);
        to_dest_uniforms.set_texture("uTexture", &texture);

        display.draw(&to_dest_vertex_buffer, &to_dest_index_buffer, &to_dest_uniforms);
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
