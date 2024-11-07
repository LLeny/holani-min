use std::path::PathBuf;
use clap::Parser;
use holani::{cartridge::lnx_header::LNXRotation, mikey::video::{LYNX_SCREEN_HEIGHT, LYNX_SCREEN_WIDTH}, suzy::registers::Joystick};
use keycodes::translate_keycode;
use miniquad::window::screen_size;
use macroquad::prelude::*;
use runner::{runner_config::RunnerConfig, Runner};

pub(crate) mod runner;
pub(crate) mod keycodes;
pub(crate) mod sound_source;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Cartright, can be .o or a .lnx file
    #[arg(short, long)]
    cartridge: PathBuf,

    /// ROM override
    #[arg(short, long)]
    rom: Option<PathBuf>,

    /// Buttons mapping <up><down><left><right><out><in><o1><o2>
    #[arg(short, long, default_value = "ikjlqw12")]
    buttons: Option<String>,

    /// Linear display filter
    #[arg(short, long, default_value_t = false)]
    linear: bool,

    /// Mute sound
    #[arg(short, long, default_value_t = false)]
    mute: bool,

    /// Enable Comlynx
    #[arg(short('x'), long, default_value_t = false)]
    comlynx: bool,
}


#[macroquad::main("Holani")]
async fn main() {
    let config = process_args();

    let mut runner = Runner::new(config.clone());
    let (input_tx, update_display_rx, rotation) = runner.initialize_thread();

    let mut joystick: Joystick = Joystick::empty();

    let mut rgba_buffer: Vec<u8> = vec![255; (LYNX_SCREEN_WIDTH * LYNX_SCREEN_HEIGHT * 4) as usize];
    let display = Texture2D::from_rgba8(LYNX_SCREEN_WIDTH as u16, LYNX_SCREEN_HEIGHT as u16, rgba_buffer.as_slice());
    display.set_filter(if config.linear_filter() { FilterMode::Linear } else { FilterMode::Nearest });

    let (rotation, ratio, zoom) = match rotation {
        LNXRotation::None => ( 0., LYNX_SCREEN_WIDTH as f32 / LYNX_SCREEN_HEIGHT as f32, vec2(2./LYNX_SCREEN_WIDTH as f32, 2./LYNX_SCREEN_HEIGHT as f32)),
        LNXRotation::_270 => (90., LYNX_SCREEN_HEIGHT as f32/ LYNX_SCREEN_WIDTH  as f32, vec2(2./LYNX_SCREEN_HEIGHT as f32, 2./LYNX_SCREEN_WIDTH as f32)),
        LNXRotation::_90 => (270., LYNX_SCREEN_HEIGHT as f32/ LYNX_SCREEN_WIDTH  as f32, vec2(2./LYNX_SCREEN_HEIGHT as f32, 2./LYNX_SCREEN_WIDTH as f32)),
    };  
    let mut render_target_camera = Camera2D {
        target: vec2(LYNX_SCREEN_WIDTH as f32/2., LYNX_SCREEN_HEIGHT as f32/2.),
        zoom,
        rotation,
        offset: vec2(0., 0.),
        render_target: Some(render_target(LYNX_SCREEN_WIDTH, LYNX_SCREEN_HEIGHT)),
        viewport: None,
    };
    
    let (mut display_width, mut display_height) = (0., 0.);
    let (mut origin_x, mut origin_y) = (0., 0.);

    loop {
        let j = joystick;                               
        config.button_mapping().iter().for_each(|btn| 
            joystick.set(*btn.1, is_key_down(*btn.0)
        ));
        if j != joystick {
            input_tx.send(joystick.bits()).unwrap();
        }

        let (dw, dh) = screen_size();
        if dw != display_width || dh != display_height {
            display_width = dw;
            display_height = dh;
            let (target_width, target_height) = if display_width / ratio > display_height {
                ((display_height * ratio) as u32, display_height as u32)
            } else {
                (display_width as u32, (display_width / ratio) as u32)
            };            
            origin_x = (display_width - target_width as f32) / 2.;
            origin_y = (display_height - target_height as f32) / 2.;
            render_target_camera.render_target = Some(render_target(target_width, target_height));
        }

        if let Ok(Some(rgb)) = update_display_rx.try_recv() {
            for (dst, src) in rgba_buffer.chunks_exact_mut(4).zip(rgb.chunks_exact(3)) {
                dst[0..3].copy_from_slice(src);
            }
            display.update_from_bytes(LYNX_SCREEN_WIDTH, LYNX_SCREEN_HEIGHT, rgba_buffer.as_slice());
        }
        set_camera(&render_target_camera);
        draw_texture(&display, 0., 0., WHITE);
        set_default_camera();
        draw_texture(&render_target_camera.render_target.as_ref().unwrap().texture, origin_x, origin_y, WHITE);

        next_frame().await
    }
}

fn process_args() -> RunnerConfig {
    let args = Args::parse();

    let mut config = RunnerConfig::new();
    if let Some(rom) = args.rom {
        config.set_rom(rom);
    }
    config.set_cartridge(args.cartridge);

    config.set_linear_filter(args.linear);
    config.set_mute(args.mute);
    config.set_comlynx(args.comlynx);

    let btns = args.buttons.unwrap();
    if btns.len() != 8 {
        panic!("Buttons mapping should be 8 characters.");
    }
    for (c, btn) in btns.chars().zip([Joystick::up, Joystick::down, Joystick::left, Joystick::right, Joystick::outside, Joystick::inside, Joystick::option_1, Joystick::option_2]) {
        let key = translate_keycode(c);
        if key == KeyCode::Unknown {
            panic!("Buttons mapping: Unknown key '{}'.", c);
        }
        config.set_button_mapping(key, btn);
    }

    config
}