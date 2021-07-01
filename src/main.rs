use argh::FromArgs;
//use glfw::Modifiers;
use glfw::{Action, Context as _, Key, Modifiers, WindowEvent};
use image::RgbaImage;
use luminance::blending::{Blending, Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{PipelineState, TextureBinding};
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::Uniform;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::{Dim2, GenMipmaps, Sampler, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{GL33Context, GlfwSurface};
use luminance_windowing::WindowOpt;

use std::cmp::min;
use std::process::exit;

// Idea: add an id to the vertex to determine v_uv instead of
// guessing based on the position

const VS: &str = include_str!("texture-vs.glsl");
const FS: &str = include_str!("texture-fs.glsl");
type GlfwBackend = <GL33Context as GraphicsContext>::Backend;

#[derive(Copy, Clone, Debug, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,
}

#[derive(Copy, Clone, Vertex, Debug)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex(VertexPosition);

#[derive(UniformInterface)]
struct ShaderInterface {
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

#[derive(FromArgs, Debug)]
/// png viewer and editor
struct PNGArgs {
    /// don't display the input image
    #[argh(switch, short = 'q')]
    quiet: bool,

    /// save to input file
    #[argh(switch, short = 'i')]
    in_place: bool,

    /// output file
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// crop left
    #[argh(option, short = 'l')]
    crop_left: Option<u32>,

    /// crop right
    #[argh(option, short = 'r')]
    crop_right: Option<u32>,

    /// crop top
    #[argh(option, short = 't')]
    crop_top: Option<u32>,

    /// crop bottom
    #[argh(option, short = 'b')]
    crop_bottom: Option<u32>,

    #[argh(positional)]
    input: String,
}

fn crop_image(
    image: &mut RgbaImage,
    crop_left: u32,
    crop_right: u32,
    crop_top: u32,
    crop_bottom: u32,
) -> RgbaImage {
    let width = image.width() - crop_left - crop_right;
    let height = image.height() - crop_top - crop_bottom;
    image::imageops::crop(image, crop_left, crop_top, width, height).to_image()
}

fn main() {
    let mut args: PNGArgs = argh::from_env();

    if args.in_place {
        match args.output {
            None => args.output = Some(args.input.clone()),
            Some(_) => {
                eprintln!("Cannot specify both --in-place and --output");
                exit(1);
            }
        }
    }

    let mut image: RgbaImage = match image::open(&args.input) {
        Ok(im) => im.into_rgba8(),
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    };

    image = crop_image(
        &mut image,
        args.crop_left.unwrap_or(0),
        args.crop_right.unwrap_or(0),
        args.crop_top.unwrap_or(0),
        args.crop_bottom.unwrap_or(0),
    );

    let output_image = if args.quiet {
        image
    } else {
        let surface = GlfwSurface::new_gl33("motsu", WindowOpt::default());
        match surface {
            Ok(surface) => main_loop(surface, &mut image),
            Err(e) => {
                eprintln!("cannot create graphics surface:\n{}", e);
                exit(1);
            }
        }
    };

    if let Some(outfile) = args.output {
        if let Err(e) = output_image.save(outfile) {
            eprintln!("{}", e);
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

fn make_texture(
    surface: &mut GlfwSurface,
    display_image: &RgbaImage,
) -> Texture<GlfwBackend, Dim2, NormRGBA8UI> {
    surface
        .context
        .new_texture_raw(
            [display_image.width() as u32, display_image.height() as u32],
            0,
            Sampler::default(),
            GenMipmaps::No,
            display_image.as_raw(),
        )
        .expect("luminance texture creation failed")
}

fn make_tess(surface: &mut GlfwSurface, display_image: &RgbaImage) -> Tess<GlfwBackend, Vertex> {
    let (width, height) = surface.context.window.get_size();
    TessBuilder::new(&mut surface.context)
        .set_vertices(calculate_vertices(
            display_image.width() as usize,
            display_image.height() as usize,
            width as u32,
            height as u32,
        ))
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap()
}

fn calculate_delta(modifiers: Modifiers) -> u32 {
    if modifiers.contains(Modifiers::Control) {
        10
    } else {
        1
    }
}

fn main_loop(mut surface: GlfwSurface, image: &mut RgbaImage) -> RgbaImage {
    // setup for loop
    //
    let mut display_image = image.clone();
    let mut redraw = false;
    let mut crop_amt_left = 0;
    let mut crop_amt_right = 0;
    let mut crop_amt_top = 0;
    let mut crop_amt_bottom = 0;

    let mut program = surface
        .context
        .new_shader_program::<(), (), ShaderInterface>()
        .from_strings(VS, None, None, FS)
        .expect("Program failed")
        .ignore_warnings();
    let render_st = RenderState::default().set_blending(Blending {
        equation: Equation::Additive,
        src: Factor::SrcAlpha,
        dst: Factor::Zero,
    });
    let pipeline_st = PipelineState::default()
        .set_clear_color([1.0, 0.0, 1.0, 1.0])
        .enable_clear_color(true);

    'app: loop {
        surface.context.window.glfw.poll_events();
        for (_, event) in surface.events_rx.try_iter() {
            // Nothing needs to happen on key release
            if let WindowEvent::Key(_, _, Action::Release, _) = event {
                continue;
            }

            // TODO: figure out a clever way to reduce code duplication
            redraw = true;
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape | Key::Q, _, _, _) => break 'app,
                WindowEvent::Key(Key::K | Key::Up, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop_amt_top -= min(delta, crop_amt_top);
                    } else {
                        crop_amt_bottom +=
                            min(delta, image.height() - crop_amt_top - crop_amt_bottom - 1);
                    }
                }
                WindowEvent::Key(Key::J | Key::Down, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop_amt_bottom -= min(delta, crop_amt_bottom);
                    } else {
                        crop_amt_top +=
                            min(delta, image.height() - crop_amt_top - crop_amt_bottom - 1);
                    }
                }
                WindowEvent::Key(Key::H | Key::Left, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop_amt_left -= min(delta, crop_amt_left);
                    } else {
                        crop_amt_right +=
                            min(delta, image.width() - crop_amt_left - crop_amt_right - 1);
                    }
                }
                WindowEvent::Key(Key::L | Key::Right, _, _, modifiers) => {
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
            display_image = crop_image(
                image,
                crop_amt_left,
                crop_amt_right,
                crop_amt_top,
                crop_amt_bottom,
            );
            let tess = make_tess(&mut surface, &display_image);
            let back_buffer = surface.context.back_buffer().unwrap();
            let mut tex = make_texture(&mut surface, &display_image);
            redraw = false;

            surface
                .context
                .new_pipeline_gate()
                .pipeline(&back_buffer, &pipeline_st, |pipeline, mut shd_gate| {
                    let bound_tex = pipeline.bind_texture(&mut tex)?;
                    shd_gate.shade(&mut program, |mut iface, uni, mut rdr_gate| {
                        iface.set(&uni.tex, bound_tex.binding());
                        //iface.tex.update(&bound_tex);
                        rdr_gate.render(&render_st, |mut tess_gate| tess_gate.render(&tess))
                    })
                })
                .assume();
            surface.context.window.swap_buffers();
        }
    }

    display_image
}
