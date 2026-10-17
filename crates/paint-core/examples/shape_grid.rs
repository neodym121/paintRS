//! Renders the shape gallery in the grid used by tools/compare/shapes_study.py, as a window
//! screenshot would show it (canvas at client (210, 201)), for offline comparison:
//! cargo run -p paint-core --example shape_grid -- out.png [width] [style] [antialias]
use paint_core::color::Rgba;
use paint_core::shape::{
    flatten, px_center, render, unit_path, PaintStyle, ShapeGeom, ShapeKind, ShapeStyle,
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|a| a == "dump") {
        // Unit outlines, one line per contour: kind x,y x,y ...
        for (k, kind) in ShapeKind::ALL.into_iter().enumerate() {
            for (c, _) in flatten(&unit_path(kind), 1) {
                let pts: Vec<String> = c.iter().map(|p| format!("{},{}", p[0], p[1])).collect();
                println!("{k} {}", pts.join(" "));
            }
        }
        return;
    }
    if args.get(1).is_some_and(|a| a == "thick") {
        // Layout of tools/compare/thick_study.py: rect, oval, line at 1/3/5/8 px.
        let out = &args[2];
        let y0: i32 = args[3].parse().unwrap();
        let antialias = args.get(4).is_some_and(|s| s == "1");
        let (ox, oy) = (210u32, 201u32);
        let (cw, ch) = (1500u32, 720u32);
        let mut img = image::RgbImage::from_pixel(ox + cw + 10, oy + ch + 10, image::Rgb([255, 255, 255]));
        for (i, width) in [1u32, 3, 5, 8].into_iter().enumerate() {
            for (j, kind) in [ShapeKind::Rectangle, ShapeKind::Oval, ShapeKind::Line].into_iter().enumerate()
            {
                let x = 40 + 360 * i as i32 + 110 * j as i32;
                let (a, b) = (px_center(x, y0), px_center(x + 80, y0 + 60));
                let geom = match kind {
                    ShapeKind::Line => ShapeGeom::Line { a, b },
                    _ => ShapeGeom::Boxed { kind, a, b, angle: 0.0 },
                };
                let style = ShapeStyle {
                    outline: PaintStyle::Solid,
                    fill: PaintStyle::None,
                    width,
                    outline_color: Rgba::BLACK,
                    fill_color: Rgba::WHITE,
                    antialias,
                };
                blit(&mut img, &geom, &style, (ox, oy), (cw, ch));
            }
        }
        img.save(out).unwrap();
        return;
    }
    if args.get(1).is_some_and(|a| a == "styles") {
        // Layout of tools/compare/style_study.py: brush outlines (8 px, black) at y=470 and
        // brush fills (red) at y=590 for rectangles and ovals.
        let out = &args[2];
        let (ox, oy) = (210u32, 201u32);
        let (cw, ch) = (1500u32, 720u32);
        let mut img = image::RgbImage::from_pixel(ox + cw + 10, oy + ch + 10, image::Rgb([255, 255, 255]));
        for (row, y) in [470, 590].into_iter().enumerate() {
            for k in 0..5 {
                for (j, kind) in [ShapeKind::Rectangle, ShapeKind::Oval].into_iter().enumerate() {
                    let x = 40 + 280 * k as i32 + 130 * j as i32;
                    let (a, b) = (px_center(x, y), px_center(x + 110, y + 90));
                    let geom = ShapeGeom::Boxed { kind, a, b, angle: 0.0 };
                    let st = PaintStyle::ALL[k + 2];
                    let style = ShapeStyle {
                        outline: if row == 0 { st } else { PaintStyle::None },
                        fill: if row == 0 { PaintStyle::None } else { st },
                        width: 8,
                        outline_color: Rgba::BLACK,
                        fill_color: Rgba::rgb(237, 28, 36),
                        antialias: false,
                    };
                    blit(&mut img, &geom, &style, (ox, oy), (cw, ch));
                }
            }
        }
        img.save(out).unwrap();
        return;
    }
    let out = args.get(1).cloned().unwrap_or_else(|| "grid.png".into());
    let width: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1);
    let outline = args
        .get(3)
        .and_then(|s| s.parse::<usize>().ok())
        .map(|i| PaintStyle::ALL[i])
        .unwrap_or(PaintStyle::Solid);
    let antialias = args.get(4).is_some_and(|s| s == "1");
    let style = ShapeStyle {
        outline,
        fill: PaintStyle::None,
        width,
        outline_color: Rgba::BLACK,
        fill_color: Rgba::WHITE,
        antialias,
    };
    let (ox, oy) = (210u32, 201u32);
    let (cw, ch) = (1500u32, 720u32);
    let mut img = image::RgbImage::from_pixel(ox + cw + 10, oy + ch + 10, image::Rgb([255, 255, 255]));
    for (k, kind) in ShapeKind::ALL.into_iter().enumerate() {
        let (r, c) = (k as i32 / 6, k as i32 % 6);
        let (x0, y0) = (250 * c + 25, 180 * r + 25);
        let (a, b) = (px_center(x0, y0), px_center(x0 + 200, y0 + 130));
        let geom = match kind {
            ShapeKind::Line => ShapeGeom::Line { a, b },
            ShapeKind::Curve | ShapeKind::Polygon => continue,
            _ => ShapeGeom::Boxed { kind, a, b, angle: 0.0 },
        };
        blit(&mut img, &geom, &style, (ox, oy), (cw, ch));
    }
    img.save(out).unwrap();
}

fn blit(img: &mut image::RgbImage, geom: &ShapeGeom, style: &ShapeStyle, o: (u32, u32), canvas: (u32, u32)) {
    let Some((ras, rect)) = render(geom, style, canvas, 7) else { return };
    for y in 0..rect.h {
        for x in 0..rect.w {
            let p = ras.get(x as i32, y as i32).unwrap().unpremultiplied();
            if p.a == 0 {
                continue;
            }
            let px = img.get_pixel_mut(o.0 + rect.x as u32 + x, o.1 + rect.y as u32 + y);
            for (ch, v) in [p.r, p.g, p.b].into_iter().enumerate() {
                px.0[ch] = ((px.0[ch] as u32 * (255 - p.a as u32) + v as u32 * p.a as u32) / 255) as u8;
            }
        }
    }
}
