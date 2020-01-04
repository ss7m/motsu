mod png;
use std::env;
use std::io;
use std::process::exit;

use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext as _;
use luminance::pipeline::BoundTexture;
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::{Mode, TessBuilder};
use luminance::texture::{Dim2, Flat, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{Action, GlfwSurface, Key, Surface as _, WindowDim, WindowEvent, WindowOpt};

const VS: &str = include_str!("texture-vs.glsl");
const FS: &str = include_str!("texture-fs.glsl");

#[derive(Copy, Clone, Debug, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,
}

#[derive(Vertex, Debug)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex(VertexPosition);

#[derive(UniformInterface)]
struct ShaderInterface {
    tex: Uniform<&'static BoundTexture<'static, Flat, Dim2, NormUnsigned>>,
}

fn main() -> io::Result<()> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Need to give input file");
        exit(1);
    }

    let png = png::PNG::new(&args[1]);

    let mut image = png.get_image();
    image.flip_vertical();
    image.convert(png::ColorType::RGBAlpha());

    println!("width: {}", png.get_width());
    println!("height: {}", png.get_height());

    let surface = GlfwSurface::new(
        WindowDim::Windowed(960, 540),
        "Hello, World!",
        WindowOpt::default(),
    );

    match surface {
        Ok(mut surface) => {
            eprintln!("graphics surface created");

            let tex: Texture<Flat, Dim2, NormRGBA8UI> = Texture::new(
                &mut surface,
                [image.width as u32, image.height as u32],
                0,
                Sampler::default(),
            )
            .expect("luminance texture creation failed");

            tex.upload_raw(GenMipmaps::No, &image.data).unwrap();

            main_loop(surface, tex, &mut image);
        }
        Err(e) => {
            eprintln!("cannot create graphics surface:\n{}", e);
            exit(1);
        }
    }

    if args.len() >= 3 {
        image.flip_vertical();
        image.write_to_file(&args[2]);
    }

    Ok(())
}

fn calculate_vertices(
    image_width: usize,
    image_height: usize,
    buffer_width: u32,
    buffer_height: u32,
) -> [Vertex; 4] {
    let image_width: f32 = image_width as f32;
    let image_height: f32 = image_height as f32;
    let buffer_width: f32 = buffer_width as f32;
    let buffer_height: f32 = buffer_height as f32;

    let width_percent = if image_width <= buffer_width {
        image_width / buffer_width
    } else {
        1.0
    };

    let height_percent = if image_height <= buffer_height {
        image_height / buffer_height
    } else {
        1.0
    };

    [
        Vertex(VertexPosition::new([-width_percent, -height_percent])),
        Vertex(VertexPosition::new([-width_percent, height_percent])),
        Vertex(VertexPosition::new([width_percent, height_percent])),
        Vertex(VertexPosition::new([width_percent, -height_percent])),
    ]
}

fn main_loop(
    mut surface: GlfwSurface,
    mut tex: Texture<Flat, Dim2, NormRGBA8UI>,
    image: &mut png::Image,
) {
    let mut back_buffer = surface.back_buffer().unwrap();

    let program = Program::<(), (), ShaderInterface>::from_strings(None, VS, None, FS)
        .expect("AAAAAAAHHHHHHHHHHHH")
        .ignore_warnings();

    let render_st =
        RenderState::default().set_blending((Equation::Additive, Factor::SrcAlpha, Factor::Zero));

    let mut tess = TessBuilder::new(&mut surface)
        .add_vertices(calculate_vertices(
            image.width,
            image.height,
            back_buffer.width(),
            back_buffer.height(),
        ))
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap();

    let mut resize = false;
    let mut image_changed = false;

    'app: loop {
        for event in surface.poll_events() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }
                WindowEvent::FramebufferSize(..) => {
                    resize = true;
                }
                WindowEvent::Key(Key::F, _, Action::Press, _) => {
                    image.flip_vertical();
                    image_changed = true;
                }
                WindowEvent::Key(Key::Up, _, action, _) => {
                    if action != Action::Release {
                        image.crop(0, 0, 1, 0);
                        image_changed = true;
                    }
                }
                WindowEvent::Key(Key::Down, _, action, _) => {
                    if action != Action::Release {
                        image.crop(0, 0, 0, 1);
                        image_changed = true;
                    }
                }
                WindowEvent::Key(Key::Left, _, action, _) => {
                    if action != Action::Release {
                        image.crop(0, 1, 0, 0);
                        image_changed = true;
                    }
                }
                WindowEvent::Key(Key::Right, _, action, _) => {
                    if action != Action::Release {
                        image.crop(1, 0, 0, 0);
                        image_changed = true;
                    }
                }
                _ => (),
            }
        }

        if resize || image_changed {
            tess = TessBuilder::new(&mut surface)
                .add_vertices(calculate_vertices(
                    image.width,
                    image.height,
                    back_buffer.width(),
                    back_buffer.height(),
                ))
                .set_mode(Mode::TriangleFan)
                .build()
                .unwrap();

            back_buffer = surface.back_buffer().unwrap();

            tex = Texture::new(
                &mut surface,
                [image.width as u32, image.height as u32],
                0,
                Sampler::default(),
            )
            .expect("luminance texture creation failed");

            tex.upload_raw(GenMipmaps::No, &image.data).unwrap();

            resize = false;
            image_changed = false;
        }

        surface.pipeline_builder().pipeline(
            &back_buffer,
            [0.0, 0.0, 0.0, 0.0],
            |pipeline, mut shd_gate| {
                let bound_tex = pipeline.bind_texture(&tex);
                shd_gate.shade(&program, |iface, mut rdr_gate| {
                    iface.tex.update(&bound_tex);
                    rdr_gate.render(render_st, |mut tess_gate| {
                        tess_gate.render(&tess);
                    });
                });
            },
        );
        surface.swap_buffers();
    }
}
