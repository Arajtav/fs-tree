#[cfg(target_family = "unix")]
fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> [f32; 3] {
    let chroma = value * saturation;
    let x = chroma * (1.0 - ((hue * 6.0) % 2.0 - 1.0).abs());
    let min = value - chroma;
    let (r, g, b) = match hue {
        h if h < 1.0 / 6.0 => (chroma, x, 0.0),
        h if h < 2.0 / 6.0 => (x, chroma, 0.0),
        h if h < 3.0 / 6.0 => (0.0, chroma, x),
        h if h < 4.0 / 6.0 => (0.0, x, chroma),
        h if h < 5.0 / 6.0 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    [r + min, g + min, b + min]
}

#[cfg(target_family = "unix")]
#[inline(always)]
fn hash(n: u32) -> u32 {
    n.wrapping_mul(0x45d9f3b).rotate_left(13)
}

pub fn get_color_from_age(now: i64, then: i64) -> [f32; 3] {
    if then > now {
        return [0.243, 0.925, 0.663];
    }

    const MAX_AGE: f32 = 5.0 * 365.0 * 24.0 * 60.0 * 60.0;

    let normalized_age = ((now - then) as f32 / MAX_AGE).min(1.0);

    let fade = 1.0 - (normalized_age * 9.0 + 1.0).log10();
    let gray = fade.clamp(0.0, 1.0);

    [gray, gray, gray]
}

#[cfg(target_family = "unix")]
pub fn get_color_from_id(id: u32, current: u32) -> [f32; 3] {
    if id == 0 {
        return [0.0, 0.0, 0.0];
    }

    if id == current {
        return [1.0, 1.0, 1.0];
    }

    let value = if id < 1000 { 0.45 } else { 0.8 };
    let hue = hash(id) as f32 / u32::MAX as f32;
    hsv_to_rgb(hue, value * 0.75, value)
}
