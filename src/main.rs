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

#[derive(Clone, Copy, Default)]
struct Crop {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

const VS: &str = include_str!("texture-vs.glsl");
const FS: &str = include_str!("texture-fs.glsl");
type GlfwBackend = <GL33Context as GraphicsContext>::Backend;

#[derive(Copy, Clone, Debug, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,

    #[sem(name = "crop_left", repr = "f32", wrapper = "CropLeft")]
    CropLeft,

    #[sem(name = "crop_right", repr = "f32", wrapper = "CropRight")]
    CropRight,

    #[sem(name = "crop_top", repr = "f32", wrapper = "CropTop")]
    CropTop,

    #[sem(name = "crop_bottom", repr = "f32", wrapper = "CropBottom")]
    CropBottom,
}

#[derive(Copy, Clone, Vertex, Debug)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex(VertexPosition, CropLeft, CropRight, CropTop, CropBottom);

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

fn crop_image(image: &mut RgbaImage, crop: Crop) -> RgbaImage {
    let width = image.width() - crop.left - crop.right;
    let height = image.height() - crop.top - crop.bottom;
    image::imageops::crop(image, crop.left, crop.top, width, height).to_image()
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
        Crop {
            left: args.crop_left.unwrap_or(0),
            right: args.crop_right.unwrap_or(0),
            top: args.crop_top.unwrap_or(0),
            bottom: args.crop_bottom.unwrap_or(0),
        },
    );

    let output_image = if args.quiet {
        image
    } else {
        let surface = GlfwSurface::new_gl33("motsu", WindowOpt::default());
        match surface {
            Ok(surface) => main_loop(surface, image),
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
    image_width: u32,
    image_height: u32,
    buffer_width: u32,
    buffer_height: u32,
    crop: Crop,
) -> [Vertex; 4] {
    let crop_left: f32 = crop.left as f32;
    let crop_right: f32 = crop.right as f32;
    let crop_top: f32 = crop.top as f32;
    let crop_bottom: f32 = crop.bottom as f32;
    let image_width: f32 = image_width as f32;
    let image_height: f32 = image_height as f32;
    let buffer_width: f32 = buffer_width as f32;
    let buffer_height: f32 = buffer_height as f32;

    let cropped_width = image_width - crop_left - crop_right;
    let cropped_height = image_height - crop_top - crop_bottom;

    let width = if cropped_width <= buffer_width {
        cropped_width / buffer_width
    } else {
        1.0
    };

    let height = if cropped_height <= buffer_height {
        cropped_height / buffer_height
    } else {
        1.0
    };

    let cl = CropLeft::new(crop_left / image_width);
    let cr = CropRight::new(1.0 - crop_right / image_width);
    let ct = CropTop::new(crop_top / image_height);
    let cb = CropBottom::new(1.0 - crop_bottom / image_height);

    [
        Vertex(VertexPosition::new([-width, -height]), cl, cr, ct, cb),
        Vertex(VertexPosition::new([-width, height]), cl, cr, ct, cb),
        Vertex(VertexPosition::new([width, height]), cl, cr, ct, cb),
        Vertex(VertexPosition::new([width, -height]), cl, cr, ct, cb),
    ]
}

fn make_texture(
    surface: &mut GlfwSurface,
    image: &RgbaImage,
) -> Texture<GlfwBackend, Dim2, NormRGBA8UI> {
    let tex = surface
        .context
        .new_texture_raw(
            [image.width() as u32, image.height() as u32],
            0,
            Sampler::default(),
            GenMipmaps::No,
            image.as_raw(),
        )
        .expect("luminance texture creation failed");
    tex
}

fn make_tess(
    surface: &mut GlfwSurface,
    image: &RgbaImage,
    crop: Crop,
) -> Tess<GlfwBackend, Vertex> {
    let (width, height) = surface.context.window.get_size();
    TessBuilder::new(&mut surface.context)
        .set_vertices(calculate_vertices(
            image.width(),
            image.height(),
            width as u32,
            height as u32,
            crop,
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

fn main_loop(mut surface: GlfwSurface, mut image: RgbaImage) -> RgbaImage {
    // setup for loop
    //
    let mut redraw = true;
    let mut crop: Crop = Default::default();

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

    let mut tex = make_texture(&mut surface, &image);

    'app: loop {
        surface.context.window.glfw.poll_events();
        for (_, event) in surface.events_rx.try_iter() {
            // Nothing needs to happen on key release
            if let WindowEvent::Key(_, _, Action::Release, _) = event {
                continue;
            }

            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape | Key::Q, _, _, _) => break 'app,
                WindowEvent::Pos(_, _) | WindowEvent::Size(_, _) => {
                    redraw = true;
                }
                WindowEvent::Key(Key::K | Key::Up, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop.top -= min(delta, crop.top);
                    } else {
                        crop.bottom += min(delta, image.height() - crop.top - crop.bottom - 1);
                    }
                    redraw = true;
                }
                WindowEvent::Key(Key::J | Key::Down, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop.bottom -= min(delta, crop.bottom);
                    } else {
                        crop.top += min(delta, image.height() - crop.top - crop.bottom - 1);
                    }
                    redraw = true;
                }
                WindowEvent::Key(Key::H | Key::Left, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop.left -= min(delta, crop.left);
                    } else {
                        crop.right += min(delta, image.width() - crop.left - crop.right - 1);
                    }
                    redraw = true;
                }
                WindowEvent::Key(Key::L | Key::Right, _, _, modifiers) => {
                    let delta = calculate_delta(modifiers);
                    if modifiers.contains(Modifiers::Shift) {
                        crop.right -= min(delta, crop.right);
                    } else {
                        crop.left += min(delta, image.width() - crop.left - crop.right - 1);
                    }
                    redraw = true;
                }
                WindowEvent::Key(Key::R, _, Action::Press, _) => {
                    crop = Default::default();
                    redraw = true;
                }
                _ => {}
            }
        }

        if redraw {
            let back_buffer = surface.context.back_buffer().unwrap();
            let tess = make_tess(&mut surface, &image, crop);
            redraw = false;

            surface
                .context
                .new_pipeline_gate()
                .pipeline(&back_buffer, &pipeline_st, |pipeline, mut shd_gate| {
                    let bound_tex = pipeline.bind_texture(&mut tex)?;
                    shd_gate.shade(&mut program, |mut iface, uni, mut rdr_gate| {
                        iface.set(&uni.tex, bound_tex.binding());
                        rdr_gate.render(&render_st, |mut tess_gate| tess_gate.render(&tess))
                    })
                })
                .assume();
            surface.context.window.swap_buffers();
        }
    }

    crop_image(&mut image, crop)
}
