use argh::FromArgs;
use glfw::{
    Action, Context as _, Key, Modifiers, MouseButton, SwapInterval, WindowEvent, WindowMode,
};
use image::RgbaImage;
use luminance::blending::{Blending, Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{PipelineState, TextureBinding};
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::Uniform;
use luminance::tess::{Mode, Tess, TessBuilder};
use luminance::texture::{Dim2, Sampler, TexelUpload, Texture};
use luminance_derive::{Semantics, UniformInterface, Vertex};
use luminance_glfw::{GL33Context, GlfwSurface, GlfwSurfaceError};

use std::cmp::{max, min};
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
/// Image viewer and cropper. Use hjkl keys to crop image.
///
/// Hold CTRL to increase cropping speed, hold shift to uncrop a side.
///
/// Press q or escape to quit, and r to undo all cropping.
///
/// You may also click twice on the image to crop with the bounding rectangle
/// of the two mouse clicks.
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

    /// scale
    #[argh(option, short = 's')]
    scale: Option<f64>,

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
        let surface = GlfwSurface::new(|glfw| {
            let (mut window, events) = glfw.with_primary_monitor(|glfw, mon| {
                let (width, height) = mon
                    .and_then(|m| m.get_video_mode())
                    .map_or((500, 500), |v| (v.width / 2, v.height / 2));
                glfw.create_window(width, height, "motsu", WindowMode::Windowed)
                    .ok_or(GlfwSurfaceError::UserError("Couldn't Open Window"))
            })?;
            window.make_current();
            window.set_all_polling(true);
            glfw.set_swap_interval(SwapInterval::Sync(1));
            Ok((window, events))
        });
        match surface {
            Ok(surface) => main_loop(surface, image),
            Err(e) => {
                eprintln!("cannot create graphics surface:\n{}", e);
                exit(1);
            }
        }
    };

    let output_image = if let Some(scale) = args.scale {
        let width = output_image.width() as f64;
        let height = output_image.height() as f64;
        image::imageops::resize(
            &output_image,
            (width * scale) as u32,
            (height * scale) as u32,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        output_image
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
            Sampler::default(),
            TexelUpload::BaseLevel {
                texels: image.as_raw(),
                mipmaps: 0,
            },
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
    let mut redraw = true;
    let mut crop: Crop = Default::default();
    let mut mouse_position: (u32, u32) = (0, 0);
    let mut mouse_click: Option<(u32, u32)> = None;

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
    let pipeline_st = PipelineState::default().set_clear_color([1.0, 1.0, 1.0, 1.0]);

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
                WindowEvent::Pos(_, _) | WindowEvent::Size(_, _) | WindowEvent::Focus(_) => {
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
                    mouse_click = None;
                    redraw = true;
                }
                WindowEvent::CursorPos(x, y) => {
                    mouse_position = (x as u32, y as u32);
                }
                WindowEvent::MouseButton(MouseButton::Button1, Action::Press, _) => {
                    let (width, height) = surface.context.window.get_size();
                    let im_width = (image.width() - crop.left - crop.right) as i32;
                    let im_height = (image.height() - crop.top - crop.bottom) as i32;
                    let disp_width = min(im_width, width);
                    let disp_height = min(im_height, height);
                    match mouse_click {
                        None => mouse_click = Some(mouse_position),
                        Some(mc) => {
                            if mc == mouse_position {
                                continue;
                            }
                            let x1: i32 = mc.0 as i32 - width / 2 + disp_width / 2;
                            let y1: i32 = mc.1 as i32 - height / 2 + disp_height / 2;
                            let x2: i32 = mouse_position.0 as i32 - width / 2 + disp_width / 2;
                            let y2: i32 = mouse_position.1 as i32 - height / 2 + disp_height / 2;

                            if x1 < 0
                                || x2 < 0
                                || y1 < 0
                                || y2 < 0
                                || x1 > disp_width
                                || x2 > disp_width
                                || y1 > disp_height
                                || y2 > disp_height
                            {
                                mouse_click = None;
                                continue;
                            }

                            let (x1, x2) = if width < im_width {
                                (x1 * im_width / width, x2 * im_width / width)
                            } else {
                                (x1, x2)
                            };

                            let (y1, y2) = if height < im_height {
                                (y1 * im_height / height, y2 * im_height / height)
                            } else {
                                (y1, y2)
                            };

                            crop.left += min(x1, x2) as u32;
                            crop.right += (im_width - max(x1, x2)) as u32;
                            crop.top += min(y1, y2) as u32;
                            crop.bottom += (im_height - max(y1, y2)) as u32;
                            mouse_click = None;
                            redraw = true;
                        }
                    }
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
