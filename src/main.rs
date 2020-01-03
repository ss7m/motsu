mod png;
use std::io;

use luminance::blending::{Equation, Factor};
use luminance::context::GraphicsContext as _;
use luminance::pipeline::BoundTexture;
use luminance::pixel::{NormRGBA8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::program::{Program, Uniform};
use luminance::tess::{Mode, TessBuilder};
use luminance::texture::{Dim2, Flat, GenMipmaps, Sampler, Texture};
use luminance_derive::UniformInterface;
use luminance_glfw::{Action, GlfwSurface, Key, Surface as _, WindowDim, WindowEvent, WindowOpt};
use std::process::exit;

const VS: &str = include_str!("texture-vs.glsl");
const FS: &str = include_str!("texture-fs.glsl");

#[derive(UniformInterface)]
struct ShaderInterface {
    tex: Uniform<&'static BoundTexture<'static, Flat, Dim2, NormUnsigned>>,
}

fn main() -> io::Result<()> {
    let png = png::PNG::new();
    png.read_file("/home/sam-barr/Pictures/bliss.png");

    let image = png
        .get_image()
        .flip_vertical()
        .convert(png::ColorType::Gray())
        .convert(png::ColorType::RGBAlpha());

    image.write_to_file("out.png");

    return Ok(());

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
                [image.width, image.height],
                0,
                Sampler::default(),
            )
            .expect("luminance texture creation failed");

            tex.upload_raw(GenMipmaps::No, &image.data).unwrap();

            main_loop(surface, tex);
        }
        Err(e) => {
            eprintln!("cannot create graphics surface:\n{}", e);
            exit(1);
        }
    }

    Ok(())
}

fn main_loop(mut surface: GlfwSurface, tex: Texture<Flat, Dim2, NormRGBA8UI>) {
    let mut back_buffer = surface.back_buffer().unwrap();

    let program = Program::<(), (), ShaderInterface>::from_strings(None, VS, None, FS)
        .expect("AAAAAAAHHHHHHHHHHHH")
        .ignore_warnings();

    let render_st =
        RenderState::default().set_blending((Equation::Additive, Factor::SrcAlpha, Factor::Zero));
    let tess = TessBuilder::new(&mut surface)
        .set_vertex_nb(4)
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap();

    let mut resize = false;

    'app: loop {
        for event in surface.poll_events() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }
                WindowEvent::FramebufferSize(..) => {
                    resize = true;
                }
                _ => (),
            }
        }

        if resize {
            back_buffer = surface.back_buffer().unwrap();
            resize = false;
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
