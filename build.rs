// SPDX-License-Identifier: GPL-3.0-or-later
//
// Embeds the application icon (`assets/icons/org.gnome.Mines.svg`) into the
// Windows `.exe` binary as a multi-resolution Win32 `RT_GROUP_ICON` resource
// without requiring external SDK tools or pre-baked binary `.ico` files.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ICON_SVG: &[u8] = include_bytes!("assets/icons/org.gnome.Mines.svg");
const ICON_SIZES: [u32; 5] = [16, 32, 48, 64, 256];

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/org.gnome.Mines.svg");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR must be set"));
    let dibs: Vec<(u32, Vec<u8>)> = ICON_SIZES
        .into_iter()
        .map(|size| (size, rasterize_icon_dib(size)))
        .collect();

    match env::var("CARGO_CFG_TARGET_ENV").as_deref() {
        Ok("gnu") => emit_gnu_resource(&out_dir, &dibs),
        _ => {
            let res_path = out_dir.join("minesweeper_icon.res");
            fs::write(&res_path, build_win32_res(&dibs)).expect("write Win32 .res icon resource");
            println!("cargo:rustc-link-arg-bins={}", res_path.display());
        }
    }
}

fn rasterize_icon_dib(size: u32) -> Vec<u8> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(ICON_SVG, &options).expect("valid app icon SVG");
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("allocate icon pixmap");
    let tree_size = tree.size();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            size as f32 / tree_size.width(),
            size as f32 / tree_size.height(),
        ),
        &mut pixmap.as_mut(),
    );

    let rgba = pixmap.data();
    let width = size as usize;
    let height = size as usize;
    let xor_bytes = width * height * 4;
    let and_stride = width.div_ceil(32) * 4;
    let and_bytes = and_stride * height;

    let mut dib = Vec::with_capacity(40 + xor_bytes + and_bytes);
    // BITMAPINFOHEADER (40 bytes)
    dib.extend_from_slice(&40u32.to_le_bytes());
    dib.extend_from_slice(&(size as i32).to_le_bytes());
    dib.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    dib.extend_from_slice(&1u16.to_le_bytes());
    dib.extend_from_slice(&32u16.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&((xor_bytes + and_bytes) as u32).to_le_bytes());
    dib.extend_from_slice(&0i32.to_le_bytes());
    dib.extend_from_slice(&0i32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());
    dib.extend_from_slice(&0u32.to_le_bytes());

    // Bottom-up straight BGRA XOR bitmap
    for y in (0..height).rev() {
        for x in 0..width {
            let offset = (y * width + x) * 4;
            let a = rgba[offset + 3];
            let (r, g, b) = if a == 0 {
                (0, 0, 0)
            } else {
                let unpremul = |c: u8| -> u8 {
                    ((u32::from(c) * 255 + u32::from(a) / 2) / u32::from(a)).min(255) as u8
                };
                (
                    unpremul(rgba[offset]),
                    unpremul(rgba[offset + 1]),
                    unpremul(rgba[offset + 2]),
                )
            };
            dib.extend_from_slice(&[b, g, r, a]);
        }
    }

    // Bottom-up 1bpp transparency AND mask
    for y in (0..height).rev() {
        let mut row = vec![0u8; and_stride];
        for x in 0..width {
            let a = rgba[(y * width + x) * 4 + 3];
            if a == 0 {
                row[x / 8] |= 0x80 >> (x % 8);
            }
        }
        dib.extend_from_slice(&row);
    }

    dib
}

fn build_win32_res(dibs: &[(u32, Vec<u8>)]) -> Vec<u8> {
    const RT_ICON: u16 = 3;
    const RT_GROUP_ICON: u16 = 14;

    let mut res = Vec::new();
    // Mandatory empty 32-bit resource file prologue record.
    write_res_record(&mut res, 0, 0, 0, 0, &[]);

    for (idx, (_, dib)) in dibs.iter().enumerate() {
        let icon_id = (idx + 1) as u16;
        write_res_record(&mut res, RT_ICON, icon_id, 0x1010, 0x0409, dib);
    }

    let mut group = Vec::with_capacity(6 + dibs.len() * 14);
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&(dibs.len() as u16).to_le_bytes());
    for (idx, (size, dib)) in dibs.iter().enumerate() {
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        group.push(dim);
        group.push(dim);
        group.push(0);
        group.push(0);
        group.extend_from_slice(&1u16.to_le_bytes());
        group.extend_from_slice(&32u16.to_le_bytes());
        group.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        group.extend_from_slice(&((idx + 1) as u16).to_le_bytes());
    }
    write_res_record(&mut res, RT_GROUP_ICON, 1, 0x1030, 0x0409, &group);
    res
}

fn write_res_record(
    out: &mut Vec<u8>,
    type_id: u16,
    name_id: u16,
    memory_flags: u16,
    language_id: u16,
    data: &[u8],
) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&type_id.to_le_bytes());
    out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    out.extend_from_slice(&name_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&memory_flags.to_le_bytes());
    out.extend_from_slice(&language_id.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(data);
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

fn emit_gnu_resource(out_dir: &Path, dibs: &[(u32, Vec<u8>)]) {
    let ico_path = out_dir.join("minesweeper.ico");
    let rc_path = out_dir.join("minesweeper.rc");
    let obj_path = out_dir.join("minesweeper_icon.o");

    let mut ico = Vec::new();
    ico.extend_from_slice(&0u16.to_le_bytes());
    ico.extend_from_slice(&1u16.to_le_bytes());
    ico.extend_from_slice(&(dibs.len() as u16).to_le_bytes());
    let mut offset = (6 + dibs.len() * 16) as u32;
    for (size, dib) in dibs {
        let dim = if *size >= 256 { 0u8 } else { *size as u8 };
        ico.push(dim);
        ico.push(dim);
        ico.push(0);
        ico.push(0);
        ico.extend_from_slice(&1u16.to_le_bytes());
        ico.extend_from_slice(&32u16.to_le_bytes());
        ico.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        ico.extend_from_slice(&offset.to_le_bytes());
        offset += dib.len() as u32;
    }
    for (_, dib) in dibs {
        ico.extend_from_slice(dib);
    }
    fs::write(&ico_path, ico).expect("write .ico file");
    let escaped_ico = ico_path.to_string_lossy().replace('\\', "/");
    fs::write(&rc_path, format!("1 ICON \"{escaped_ico}\"\n")).expect("write .rc file");

    for tool in ["x86_64-w64-mingw32-windres", "windres"] {
        if Command::new(tool)
            .args([rc_path.as_os_str(), obj_path.as_os_str()])
            .status()
            .is_ok_and(|status| status.success())
        {
            println!("cargo:rustc-link-arg-bins={}", obj_path.display());
            break;
        }
    }
}
