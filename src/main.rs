mod png;
use png::{ColorType, Image, PNG};
use std::cmp::min;
use std::env;
use std::process::exit;

use glfw::Modifiers;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext as _;
use luminance::pipeline::{BoundTexture, PipelineState};
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::{Mode, TessBuilder};
use luminance::texture::{Dim2, Flat, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{Action, GlfwSurface, Key, Surface as _, WindowDim, WindowEvent, WindowOpt};

// Idea: add an id to the vertex to determine v_uv instead of
// guessing based on the position

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

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() < 2 {
        eprintln!("Need to give input file");
        exit(1);
    }

    let png = match PNG::new(&args[1]) {
        Ok(png) => png,
        Err(msg) => {
            eprintln!("{}", msg);
            exit(1)
        }
    };

    let mut image = png.get_image();
    image = image.convert(ColorType::RGBAlpha());

    let surface = GlfwSurface::new(
        WindowDim::Windowed(960, 540),
        "Hello, World!",
        WindowOpt::default(),
    );

    match surface {
        Ok(surface) => {
            image = main_loop(surface, image);
            if args.len() > 2 {
                image.write_to_file(&args[2]);
            }
        }
        Err(e) => {
            eprintln!("cannot create graphics surface:\n{}", e);
            exit(1);
        }
    }
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

fn main_loop(mut surface: GlfwSurface, image: Image) -> Image {
    // setup for loop
    let mut display_image = image.clone();
    let mut redraw = false;
    let mut crop_amt_left = 0;
    let mut crop_amt_right = 0;
    let mut crop_amt_top = 0;
    let mut crop_amt_bottom = 0;

    let mut tex: Texture<Flat, Dim2, NormRGBA8UI> = Texture::new(
        &mut surface,
        [display_image.width as u32, display_image.height as u32],
        0,
        Sampler::default(),
    )
    .expect("luminance texture creation failed");
    tex.upload_raw(GenMipmaps::No, &display_image.data).unwrap();

    let mut back_buffer = surface.back_buffer().unwrap();
    let program = Program::<(), (), ShaderInterface>::from_strings(None, VS, None, FS)
        .expect("AAAAAAAHHHHHHHHHHHH")
        .ignore_warnings();
    let render_st =
        RenderState::default().set_blending((Equation::Additive, Factor::SrcAlpha, Factor::Zero));
    let mut tess = TessBuilder::new(&mut surface)
        .add_vertices(calculate_vertices(
            display_image.width,
            display_image.height,
            back_buffer.width(),
            back_buffer.height(),
        ))
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap();

    'app: loop {
        for event in surface.poll_events() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }
                WindowEvent::FramebufferSize(..) => {
                    redraw = true;
                }
                WindowEvent::Key(Key::Up, _, action, modifiers) => {
                    if action != Action::Release {
                        let delta = if modifiers.contains(Modifiers::Control) {
                            10
                        } else {
                            1
                        };
                        if modifiers.contains(Modifiers::Shift) {
                            crop_amt_top = if crop_amt_top <= delta {
                                0
                            } else {
                                crop_amt_top - delta
                            };
                        } else {
                            crop_amt_bottom += delta;
                        }
                        redraw = true;
                    }
                }
                WindowEvent::Key(Key::Down, _, action, modifiers) => {
                    if action != Action::Release {
                        let delta = if modifiers.contains(Modifiers::Control) {
                            10
                        } else {
                            1
                        };
                        if modifiers.contains(Modifiers::Shift) {
                            crop_amt_bottom = if crop_amt_bottom <= delta {
                                0
                            } else {
                                crop_amt_bottom - delta
                            };
                        } else {
                            crop_amt_top += delta;
                        }
                        redraw = true;
                    }
                }
                WindowEvent::Key(Key::Left, _, action, modifiers) => {
                    if action != Action::Release {
                        let delta = if modifiers.contains(Modifiers::Control) {
                            10
                        } else {
                            1
                        };
                        if modifiers.contains(Modifiers::Shift) {
                            crop_amt_left = if crop_amt_left <= delta {
                                0
                            } else {
                                crop_amt_left - delta
                            };
                        } else {
                            crop_amt_right += delta;
                        }
                        redraw = true;
                    }
                }
                WindowEvent::Key(Key::Right, _, action, modifiers) => {
                    if action != Action::Release {
                        let delta = if modifiers.contains(Modifiers::Control) {
                            10
                        } else {
                            1
                        };
                        if modifiers.contains(Modifiers::Shift) {
                            crop_amt_right = if crop_amt_right <= delta {
                                0
                            } else {
                                crop_amt_right - delta
                            };
                        } else {
                            crop_amt_left += delta;
                        }
                        redraw = true;
                    }
                }
                _ => (),
            }
        }

        crop_amt_left = min(crop_amt_left, image.width);
        crop_amt_right = min(crop_amt_right, image.width);
        crop_amt_top = min(crop_amt_top, image.height);
        crop_amt_bottom = min(crop_amt_bottom, image.height);

        if redraw {
            display_image =
                image.crop(crop_amt_left, crop_amt_right, crop_amt_top, crop_amt_bottom);

            tess = TessBuilder::new(&mut surface)
                .add_vertices(calculate_vertices(
                    display_image.width,
                    display_image.height,
                    back_buffer.width(),
                    back_buffer.height(),
                ))
                .set_mode(Mode::TriangleFan)
                .build()
                .unwrap();

            back_buffer = surface.back_buffer().unwrap();

            tex = Texture::new(
                &mut surface,
                [display_image.width as u32, display_image.height as u32],
                0,
                Sampler::default(),
            )
            .expect("luminance texture creation failed");

            tex.upload_raw(GenMipmaps::No, &display_image.data).unwrap();

            redraw = false;
        }

        surface.pipeline_builder().pipeline(
            &back_buffer,
            &PipelineState::default(),
            |pipeline, mut shd_gate| {
                let bound_tex = pipeline.bind_texture(&tex);
                shd_gate.shade(&program, |iface, mut rdr_gate| {
                    iface.tex.update(&bound_tex);
                    rdr_gate.render(&render_st, |mut tess_gate| {
                        tess_gate.render(&tess);
                    });
                });
            },
        );
        surface.swap_buffers();
    }

    display_image
}
