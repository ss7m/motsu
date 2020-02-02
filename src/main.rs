mod image;
mod pixel;
mod png;

use argh::FromArgs;
use glfw::Modifiers;
use image::*;
use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext as _;
use luminance::pipeline::{BoundTexture, PipelineState};
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::{Dim2, Flat, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{Action, GlfwSurface, Key, Surface as _, WindowDim, WindowEvent, WindowOpt};
use pixel::*;
use std::cmp::min;
use std::process::exit;

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

#[derive(FromArgs, Debug)]
/// png viewer and editor
struct PNGArgs {
    /// don't display the input image
    #[argh(switch, short = 'q')]
    quiet: bool,

    /// output file
    #[argh(option, short = 'o')]
    output: Option<String>,

    #[argh(positional)]
    input: String,
}

fn main() {
    let args: PNGArgs = argh::from_env();

    let image = match png::load_image_from_png(&args.input) {
        Ok(image) => image,
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    let output_image = if args.quiet {
        image
    } else {
        let surface = GlfwSurface::new(
            WindowDim::Windowed(image.width() as u32, image.height() as u32),
            "PNG",
            WindowOpt::default(),
        );

        match surface {
            Ok(surface) => main_loop(surface, image),
            Err(e) => {
                eprintln!("cannot create graphics surface:\n{}", e);
                exit(1);
            }
        }
    };

    if let Some(output) = args.output {
        if let Err(e) = png::write_image_to_png(&output, output_image) {
            eprintln!("{}", e);
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

fn make_texture(
    surface: &mut GlfwSurface,
    display_image: &Image<RGBA>,
) -> Texture<Flat, Dim2, NormRGBA8UI> {
    let tex = Texture::new(
        surface,
        [display_image.width() as u32, display_image.height() as u32],
        0,
        Sampler::default(),
    )
    .expect("luminance texture creation failed");
    tex.upload_raw(GenMipmaps::No, &display_image.data())
        .unwrap();
    tex
}

fn make_tess(surface: &mut GlfwSurface, display_image: &Image<RGBA>) -> Tess {
    let width = surface.width();
    let height = surface.height();
    TessBuilder::new(surface)
        .add_vertices(calculate_vertices(
            display_image.width(),
            display_image.height(),
            width,
            height,
        ))
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap()
}

fn calculate_delta(modifiers: Modifiers) -> usize {
    if modifiers.contains(Modifiers::Control) {
        10
    } else {
        1
    }
}

fn main_loop(mut surface: GlfwSurface, image: Image<RGBA>) -> Image<RGBA> {
    // setup for loop
    //
    let mut display_image = image.clone();
    let mut redraw = false;
    let mut crop_amt_left = 0;
    let mut crop_amt_right = 0;
    let mut crop_amt_top = 0;
    let mut crop_amt_bottom = 0;

    let program = Program::<(), (), ShaderInterface>::from_strings(None, VS, None, FS)
        .expect("Program failed")
        .ignore_warnings();
    let render_st =
        RenderState::default().set_blending((Equation::Additive, Factor::SrcAlpha, Factor::Zero));

    'app: loop {
        for event in surface.poll_events() {
            // Nothing needs to happen on key release
            if let WindowEvent::Key(_, _, Action::Release, _) = event {
                continue;
            }

            // TODO: figure out a clever way to reduce code duplication
            redraw = true;
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, _, _) => break 'app,
                WindowEvent::Key(Key::Up, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop_amt_top -= min(delta, crop_amt_top);
                    } else {
                        crop_amt_bottom +=
                            min(delta, image.height() - crop_amt_top - crop_amt_bottom - 1);
                    }
                }
                WindowEvent::Key(Key::Down, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop_amt_bottom -= min(delta, crop_amt_bottom);
                    } else {
                        crop_amt_top +=
                            min(delta, image.height() - crop_amt_top - crop_amt_bottom - 1);
                    }
                }
                WindowEvent::Key(Key::Left, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop_amt_left -= min(delta, crop_amt_left);
                    } else {
                        crop_amt_right +=
                            min(delta, image.width() - crop_amt_left - crop_amt_right - 1);
                    }
                }
                WindowEvent::Key(Key::Right, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop_amt_right -= min(delta, crop_amt_right);
                    } else {
                        crop_amt_left +=
                            min(delta, image.width() - crop_amt_left - crop_amt_right - 1);
                    }
                }
                WindowEvent::Key(Key::R, _, Action::Press, _) => {
                    crop_amt_left = 0;
                    crop_amt_right = 0;
                    crop_amt_top = 0;
                    crop_amt_bottom = 0;
                }
                _ => {}
            }
        }

        if redraw {
            display_image =
                image.crop(crop_amt_left, crop_amt_right, crop_amt_top, crop_amt_bottom);
            let tess = make_tess(&mut surface, &display_image);
            let back_buffer = surface.back_buffer().unwrap();
            let tex = make_texture(&mut surface, &display_image);
            redraw = false;

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
    }

    display_image
}
