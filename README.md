# simple-gl

**simple-gl** is a simple OpenGL wrapper in Rust that you can use if you want things to *just work*.

## Installation

```toml
[dependencies.simple_gl]
git = "http://github.com/tomaka/simple-gl-rs"
```

## [Documentation](http://rust-ci.org/tomaka/simple-gl-rs/doc/simple_gl/index.html)

Everything is explained in the documentation.

## Example

```rust
#![feature(phase)]

#[phase(plugin)]
extern crate simple_gl_macros;

extern crate gl_init;
extern crate simple_gl;

#[vertex_format]
struct Vertex {
    iPosition: [f32, ..2]
}

static VERTEX_SRC: &'static str = "
    #version 110

    uniform mat4 uMatrix;

    attribute vec2 iPosition;

    void main() {
        gl_Position = vec4(iPosition, 0.0, 1.0) * uMatrix;
    }
";

static FRAGMENT_SRC: &'static str = "
    #version 110

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
            Vertex { iPosition: [-1.0, -1.0] },
            Vertex { iPosition: [-1.0,  1.0] },
            Vertex { iPosition: [ 1.0,  1.0] },
            Vertex { iPosition: [ 1.0, -1.0] }
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
        display.draw(&vb, &ib, &uniforms);
        display.end_frame();
        timer::sleep(17);

        for event in display.recv().move_iter() {
            match event {
                gl_init::Closed => break 'main,
                _ => ()
            }
        }
    }
}
```

## Note

The `#[vertex_format]` syntax extension was shamefully copied from [gfx-rs](https://github.com/gfx-rs/gfx-rs).
