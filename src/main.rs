use std::{
    env,
    f32::consts::E,
    ffi::{c_void, CString},
    fs,
};

use ffi::{
    GetMonitorWidth, GetRenderHeight, GetRenderWidth, ImageBlurGaussian, InitAudioDevice,
    LoadMusicStream, PlayMusicStream, SetMusicVolume, SetTextureFilter, UpdateMusicStream,
};
use image::{imageops::FilterType::Lanczos3, GenericImageView, ImageReader};
use raylib::{ffi::TraceLogLevel, prelude::*};

fn sigmoid(x: f32) -> f32 {
    let t = 10.0 * x - 5.0;
    E.powf(t) / (E.powf(t) + 1.0)
}

fn size_ease_in(t: f32) -> f32 {
    let t = t * 25.0 - 0.01;
    1.025 * (1.0 - E.powf(-t) + 0.01)
}
fn size_ease_out(t: f32) -> f32 {
    let t = -t * 50.0 + 50.0;
    // let t = t * 10.0 - 9.5;
    let t2 = sigmoid(-t / 4.0);
    (1.0 - t2) * E.powf(-t) + t2 * 15.0 + size_ease_in(0.5)
    // (E.powf(-t) + ease_in(0.5)).min(10.0)
}

fn size_ease(t: f32) -> f32 {
    return if t < 0.5 {
        size_ease_in(t)
    } else {
        size_ease_out(t)
    } + 0.05 * t;
}

fn alpha_ease_in(t: f32) -> f32 {
    let t = t * 50.0;
    1.0 - E.powf(-t) + 0.01
}

fn alpha_ease_out(t: f32) -> f32 {
    let t = 1.0 - t + 2.0;
    -E.powf(-t) + alpha_ease_in(1.0)
}

fn alpha_ease(t: f32) -> f32 {
    if t < 1.0 {
        alpha_ease_in(t)
    } else {
        alpha_ease_out(t)
    }
}

fn blur_ease(t: f32) -> f32 {
    let t = -t * 25.0 + 25.0;
    E.powf(-t).min(250.0)
}

fn show_best_images(best: &Vec<&str>, song_path: Option<&String>, bpm: f64) {
    let photo_time = 60.0 / bpm;
    // let mut a = Command::new("mpv")
    //     .args(best.iter())
    //     .arg("--fullscreen")
    //     .arg(format!("--speed={}", speed))
    //     .spawn()
    //     .expect("boh");

    // if let Some(song) = song_path {
    //     let mut b = Command::new("mpv").arg(song).spawn().expect("boh");

    //     a.wait().unwrap();
    //     b.kill().unwrap();
    // }

    let (mut rl, thread) = raylib::init()
        .size(720, 540)
        .title("Paparazzi")
        .fullscreen()
        .resizable()
        .vsync()
        .log_level(TraceLogLevel::LOG_WARNING)
        .build();

    {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);
    }

    // rl.set_target_fps(60);

    let mut textures = Vec::new();

    let music = if let Some(song) = song_path {
        unsafe {
            InitAudioDevice();

            let music =
                LoadMusicStream(CString::new(song.to_owned()).unwrap_or_default().into_raw());
            PlayMusicStream(music);
            Some(music)
        }
    } else {
        None
    };

    // let max_screen_dim = unsafe {
    //     let monitor_id = get_current_monitor();
    //     GetMonitorWidth(monitor_id).max(get_monitor_height(monitor_id))
    // } as f32;

    let total = best.len();
    for (i, path) in best.into_iter().enumerate() {
        if let Ok(img_) = ImageReader::open(path) {
            if let Ok(img) = img_.decode() {
                // let dims = img.dimensions();
                // let min_dim = dims.0.min(dims.1) as f32;
                // let scale_factor = (max_screen_dim * 1.2 / min_dim).min(1.0);
                // println!("PRE-PRE-FIRST OK");
                let scaled = img; //img.resize(
                                  //     (dims.0 as f32 * scale_factor).round() as u32,
                                  //     (dims.1 as f32 * scale_factor).round() as u32,
                                  //     Lanczos3,
                                  // );
                let bytes_ = scaled.to_rgb8();
                let mut bytes = bytes_.as_raw().clone();
                let mut bytes2 = bytes_.as_raw().clone();

                // not eliminating unwrap because do not want to mess with mem::forget
                // should work fine anyway...

                let rimg = unsafe {
                    Image::from_raw(raylib::ffi::Image {
                        data: bytes.as_mut_ptr() as *mut c_void,
                        width: scaled.width() as i32,
                        height: scaled.height() as i32,
                        mipmaps: 1,
                        format: PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8 as i32,
                    })
                };

                let texture = rl.load_texture_from_image(&thread, &rimg).unwrap();
                unsafe {
                    SetTextureFilter(
                        texture.clone(),
                        TextureFilter::TEXTURE_FILTER_BILINEAR as i32,
                    )
                };
                let mut blurred = unsafe {
                    Image::from_raw(raylib::ffi::Image {
                        data: bytes2.as_mut_ptr() as *mut c_void,
                        width: scaled.width() as i32,
                        height: scaled.height() as i32,
                        mipmaps: 1,
                        format: PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8 as i32,
                    })
                };
                unsafe { ImageBlurGaussian(blurred.as_mut() as *mut raylib::ffi::Image, 100) };
                let texture_blurred = rl.load_texture_from_image(&thread, &blurred).unwrap();
                unsafe {
                    SetTextureFilter(
                        texture_blurred.clone(),
                        TextureFilter::TEXTURE_FILTER_BILINEAR as i32,
                    )
                };
                std::mem::forget(bytes);
                std::mem::forget(bytes2);
                std::mem::forget(rimg);
                std::mem::forget(blurred);
                textures.push((texture, texture_blurred));
            }
        }

        {
            let percentage = (i + 1) as f32 / total as f32;
            let mut d = rl.begin_drawing(&thread);
            d.clear_background(Color::BLACK);
            let w = d.get_render_width();
            let h = unsafe { GetRenderHeight() };
            let height = 20;
            d.draw_rectangle(
                0,
                h - height,
                (percentage * w as f32) as i32,
                height,
                Color::WHITE.alpha(0.05),
            );
        }
    }

    let textures = textures.into_iter().enumerate().collect::<Vec<_>>();

    let mut w;
    let mut h;

    let start_time = rl.get_time();

    let mut blur_shader = rl.load_shader_from_memory(
        &thread,
        None,
        Some(
            "#version 330
in vec2 fragTexCoord;
in vec4 fragColor;

// Input uniform values
uniform sampler2D texture0;
uniform vec4 colDiffuse;

// Output fragment color
out vec4 finalColor;

// NOTE: Add here your custom variables

// NOTE: Render size values must be passed from code
uniform float width;
uniform float height;
uniform float radius;

float offset[3] = float[](0.0, 1.3846153846, 3.2307692308);
float weight[3] = float[](0.2270270270, 0.3162162162, 0.0702702703);

const float TAU = 6.28318530718;

const float QUALITY = 2.0;
const float DIRECTIONS = 8.0;

void main()
{
    // Texel color fetching from texture sampler
    float r = radius/max(width, height);
    vec3 texelColor = texture(texture0, fragTexCoord).rgb;
    
    float x = 1.0;
    for(float d = 0.0; d<TAU; d += TAU/DIRECTIONS )
    {
        for( float i=1.0/QUALITY;i<=1.0;i+=1.0/QUALITY )
        {
            float w = i;
            texelColor += texture2D(texture0, fragTexCoord+vec2(cos(d),sin(d))*r*w).rgb;
            x += 1.0;
        }
        // x += 1.0;
    }
    texelColor = texelColor / x;

    finalColor = vec4(texelColor.r, texelColor.g, texelColor.b, 1.0)*fragColor;
}",
        ),
    );
    let uniform_width = blur_shader.get_shader_location("width");
    let uniform_height = blur_shader.get_shader_location("height");
    let uniform_radius = blur_shader.get_shader_location("radius");

    while !rl.window_should_close() {
        let time = rl.get_time() - start_time;
        let index = (time / photo_time).floor() as usize;

        if index >= textures.len() {
            if let Some(music) = music {
                unsafe { SetMusicVolume(music, 0.0) };
            }
            break;
        }

        if let Some(music) = music {
            unsafe {
                UpdateMusicStream(music);
                if index == 0 {
                    let t = (time / photo_time).fract();
                    let v = t;
                    SetMusicVolume(music, v as f32);
                } else if index >= textures.len() - 1 {
                    let t = 1.0 - (time / photo_time).fract();
                    let v = t;
                    SetMusicVolume(music, v as f32);
                } else if index >= textures.len() {
                    SetMusicVolume(music, 0.0);
                } else {
                    SetMusicVolume(music, 1.0);
                }
            }
        }

        (w, h) = unsafe { (GetRenderWidth(), GetRenderHeight()) };
        // let xs = [w as f32, h as f32, 20.0 as f32];
        // blur_shader.set_shader_value_v(uniform_size, &xs);
        blur_shader.set_shader_value(uniform_width, w as f32);
        blur_shader.set_shader_value(uniform_height, h as f32);

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
        let to_skip = ((time / photo_time) - 1.0).max(0.0).floor() as usize;
        for (i, (texture, blurred)) in textures.iter().skip(to_skip).take(2) {
            let w = w as f32;
            let h = h as f32;
            let t = ((time - *i as f64 * photo_time) / photo_time).max(0.0) as f32;
            let a = alpha_ease(t);
            let b = blur_ease(t);
            if a < 1e-6 {
                continue;
            }
            let img_w = texture.width as f32;
            let img_h = texture.height as f32;
            let (factor, scale, original_scale) = {
                let scale_x = w / img_w;
                let scale_y = h / img_h;
                let scale = scale_x.max(scale_y);
                let s = size_ease(t);
                (s, s * scale, scale)
            };

            if factor < 1.0 {
                let a = t * 2.0;
                if a > 1e-4 && a <= 1.0 {
                    d.draw_texture_ex(
                        blurred,
                        rvec2(
                            w / 2.0 - original_scale * img_w * 0.5,
                            h / 2.0 - original_scale * img_h * 0.5,
                        ),
                        0.0,
                        original_scale,
                        Color::WHITE.alpha(a),
                    );
                }
            }

            // continue;

            if b < 1.0 {
                d.draw_texture_ex(
                    texture,
                    rvec2(w / 2.0 - scale * img_w * 0.5, h / 2.0 - scale * img_h * 0.5),
                    0.0,
                    scale,
                    Color::WHITE.alpha(a),
                );
            } else {
                blur_shader.set_shader_value(uniform_radius, b);
                let mut sd = d.begin_shader_mode(&blur_shader);
                sd.draw_texture_ex(
                    texture,
                    rvec2(w / 2.0 - scale * img_w * 0.5, h / 2.0 - scale * img_h * 0.5),
                    0.0,
                    scale,
                    Color::WHITE.alpha(a),
                );
            }
        }

        if d.is_key_down(KeyboardKey::KEY_F3) {
            d.draw_text(&format!("{}", d.get_fps()), 0, 0, 50, Color::WHITE);
        }
    }
}

const DEFAULT_BPM: f64 = 12.0;

fn main() {
    let args = env::args().collect::<Vec<_>>();
    // println!("{:?}", args);
    let images_path = args.get(1).expect("No paths file provided.");
    let music_path = args.get(2);
    let bpm = args
        .get(3)
        .map(|x| x.parse::<f64>().unwrap_or(DEFAULT_BPM))
        .unwrap_or(DEFAULT_BPM);

    // println!("{} {:?} {}", images_path, music_path, bpm);
    let contents = fs::read_to_string(images_path).expect("File provided does not exist");
    let best_images = contents.lines().collect::<Vec<_>>();
    show_best_images(&best_images, music_path, bpm);
}
