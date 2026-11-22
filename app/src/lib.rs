// Mi Garaje — garaje digital para tu moto.
// Estética "dark premium" automotriz: negro carbón, superficies gris oscuro y
// un único acento rojo deportivo. Todo se dibuja a mano para controlar el
// detalle: jerarquía, espaciados, iconos y microanimaciones.
//
// La app arranca vacía: aquí solo vive la interfaz. Los datos, los cálculos y
// el guardado en disco están en `datos.rs`.

#![allow(float_literal_f32_fallback)]

mod datos;
mod entender;
#[cfg(target_os = "android")]
mod ime;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use datos::{Datos, Documento, Perfil, Servicio, Tanqueo, TipoMov, Vehiculo};
use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Id, Pos2, Rect, Rounding, Sense, Stroke, Vec2,
};

// ------------------------------------------------------------------ lienzo
const BASE_W: f32 = 400.0;
const BASE_H: f32 = 866.0;

// ------------------------------------------------------------------ paleta
const BG: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x10);
const SURF: Color32 = Color32::from_rgb(0x13, 0x17, 0x1D);
const SURF2: Color32 = Color32::from_rgb(0x1A, 0x1F, 0x27);
const SURF3: Color32 = Color32::from_rgb(0x24, 0x2A, 0x35);
const ACCENT: Color32 = Color32::from_rgb(0xE5, 0x39, 0x35);
const ACCENT_DEEP: Color32 = Color32::from_rgb(0x9C, 0x22, 0x20);
const TEXT: Color32 = Color32::from_rgb(0xF5, 0xF5, 0xF5);
const MUTED: Color32 = Color32::from_rgb(0x92, 0x99, 0xA3);
const DIM: Color32 = Color32::from_rgb(0x60, 0x68, 0x74);
const LINE: Color32 = Color32::from_rgb(0x24, 0x2A, 0x33);
const GREEN: Color32 = Color32::from_rgb(0x3F, 0xB9, 0x50);
const AMBER: Color32 = Color32::from_rgb(0xD8, 0x9B, 0x2A);

// ------------------------------------------------------------------ fuentes
fn fam(name: &str) -> FontFamily {
    FontFamily::Name(name.into())
}
const REG: &str = "reg";
const MED: &str = "med";
const SEMI: &str = "semi";
const BOLD: &str = "bold";
const NUM: &str = "num";

// ------------------------------------------------------------------ formato
fn grupos(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push('.');
        }
        out.push(ch);
    }
    if n < 0 {
        out.insert(0, '-');
    }
    out
}
fn money(v: f64) -> String {
    format!("$ {}", grupos(v.round() as i64))
}
fn km(v: i64) -> String {
    format!("{} km", grupos(v))
}
fn solo_digitos(s: &str) -> i64 {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    d.parse().unwrap_or(0)
}
fn a_dec(s: &str) -> f64 {
    s.trim().replace(',', ".").parse().unwrap_or(0.0)
}
fn fmt_int(v: i64) -> String {
    format!("{v}")
}
fn fmt_dec(v: f64) -> String {
    if (v.fract()).abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{v:.1}")
    }
}

// ------------------------------------------------------------------ mates
fn a(c: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), (c.a() as f32 * k) as u8)
}
fn mix(x: Color32, y: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (x.r() as f32 + (y.r() as f32 - x.r() as f32) * t) as u8,
        (x.g() as f32 + (y.g() as f32 - x.g() as f32) * t) as u8,
        (x.b() as f32 + (y.b() as f32 - x.b() as f32) * t) as u8,
        (x.a() as f32 + (y.a() as f32 - x.a() as f32) * t) as u8,
    )
}
fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// curva suave 0..1, para que los bordes de la foto no corten en seco
fn smooth01(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ------------------------------------------------------------------ lienzo
#[derive(Clone)]
struct Canvas {
    p: egui::Painter,
    o: Pos2,
    s: f32,
    fade: f32,
    dy: f32,
    /// Ancho de una columna de contenido, en unidades de diseño (400).
    cw: f32,
    /// Distancia horizontal entre el inicio de dos columnas contiguas.
    paso: f32,
}

impl Canvas {
    fn pos(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.o.x + x * self.s, self.o.y + (y + self.dy) * self.s)
    }
    fn rect(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_size(self.pos(x, y), Vec2::new(w * self.s, h * self.s))
    }
    fn px(&self, v: f32) -> f32 {
        v * self.s
    }
    /// Sub-lienzo desplazado a la columna `i` (0 = la propia columna).
    fn col(&self, i: usize) -> Canvas {
        let mut c = self.clone();
        c.o.x += i as f32 * self.paso * self.s;
        c
    }
    fn with(&self, fade: f32, dy: f32) -> Canvas {
        Canvas {
            p: self.p.clone(),
            o: self.o,
            s: self.s,
            fade: self.fade * fade.clamp(0.0, 1.0),
            dy: self.dy + dy,
            cw: self.cw,
            paso: self.paso,
        }
    }
    fn ink(&self, color: Color32, k: f32) -> Color32 {
        a(color, self.fade * k)
    }
}

fn txt(c: &Canvas, x: f32, y: f32, s: &str, size: f32, color: Color32, family: &str) {
    c.p.text(
        c.pos(x, y),
        Align2::LEFT_TOP,
        s,
        FontId::new(c.px(size.max(9.3)), fam(family)),
        color,
    );
}
fn txt_r(c: &Canvas, x: f32, y: f32, s: &str, size: f32, color: Color32, family: &str) {
    c.p.text(
        c.pos(x, y),
        Align2::RIGHT_TOP,
        s,
        FontId::new(c.px(size.max(9.3)), fam(family)),
        color,
    );
}
fn txt_c(c: &Canvas, x: f32, y: f32, s: &str, size: f32, color: Color32, family: &str) {
    c.p.text(
        c.pos(x, y),
        Align2::CENTER_CENTER,
        s,
        FontId::new(c.px(size.max(9.3)), fam(family)),
        color,
    );
}
/// Texto centrado en coordenadas de pantalla (para rects ya calculados).
fn txt_sc(c: &Canvas, centro: Pos2, s: &str, size: f32, color: Color32, family: &str) {
    c.p.text(
        centro,
        Align2::CENTER_CENTER,
        s,
        FontId::new(c.px(size.max(9.3)), fam(family)),
        color,
    );
}
// ------------------------------------------------------------------ formas
fn card(c: &Canvas, r: Rect, radius: f32, fill: Color32, border_k: f32) {
    c.p.rect_filled(r, Rounding::same(c.px(radius)), fill);
    if border_k > 0.0 {
        c.p.rect_stroke(
            r,
            Rounding::same(c.px(radius)),
            Stroke::new(1.0, c.ink(LINE, border_k)),
        );
        let rad = c.px(radius * 0.7);
        if r.width() > rad * 2.0 {
            let top_line = [
                Pos2::new(r.min.x + rad, r.min.y + c.px(1.0)),
                Pos2::new(r.max.x - rad, r.min.y + c.px(1.0)),
            ];
            c.p.line_segment(top_line, Stroke::new(1.0, c.ink(Color32::WHITE, 0.05)));
        }
    }
}

/// Sombra suave: unas capas de rectángulos redondeados cada vez más difusos.
fn shadow(c: &Canvas, r: Rect, radius: f32, strength: f32) {
    for i in 0..5 {
        let g = (i + 1) as f32;
        let grow = c.px(g * 1.6);
        let rr = Rect::from_min_max(
            Pos2::new(r.left() - grow, r.top() - grow + c.px(2.0)),
            Pos2::new(r.right() + grow, r.bottom() + grow + c.px(3.0)),
        );
        c.p.rect_filled(
            rr,
            Rounding::same(c.px(radius + g * 1.6)),
            c.ink(Color32::BLACK, strength * (0.16 / g)),
        );
    }
}

fn grad_v(c: &Canvas, r: Rect, top: Color32, bot: Color32) {
    let bands = 28;
    for i in 0..bands {
        let t0 = i as f32 / bands as f32;
        let t1 = (i + 1) as f32 / bands as f32;
        let y0 = r.top() + r.height() * t0;
        let y1 = r.top() + r.height() * t1 + 0.5;
        c.p.rect_filled(
            Rect::from_min_max(Pos2::new(r.left(), y0), Pos2::new(r.right(), y1)),
            Rounding::ZERO,
            mix(top, bot, (t0 + t1) * 0.5),
        );
    }
}

fn divider(c: &Canvas, x0: f32, x1: f32, y: f32, k: f32) {
    c.p.line_segment(
        [c.pos(x0, y), c.pos(x1, y)],
        Stroke::new(1.0, c.ink(LINE, k)),
    );
}

fn capsule(c: &Canvas, x: f32, y: f32, w: f32, h: f32, fill: Color32, border: Option<Color32>) {
    let r = c.rect(x, y, w, h);
    c.p.rect_filled(r, Rounding::same(c.px(h * 0.5)), fill);
    if let Some(b) = border {
        c.p.rect_stroke(r, Rounding::same(c.px(h * 0.5)), Stroke::new(1.0, b));
    }
}

fn chip(c: &Canvas, x: f32, y: f32, w: f32, h: f32, label: &str, fg: Color32, bg_k: f32) {
    capsule(c, x, y, w, h, c.ink(fg, bg_k), Some(c.ink(fg, 0.35)));
    txt_c(c, x + w * 0.5, y + h * 0.5, label, 8.6, c.ink(fg, 0.95), SEMI);
}

fn bar(c: &Canvas, x: f32, y: f32, w: f32, h: f32, t: f32, color: Color32) {
    let track = c.rect(x, y, w, h);
    c.p.rect_filled(track, Rounding::same(c.px(h * 0.5)), c.ink(SURF3, 0.9));
    let fw = (w * t.clamp(0.0, 1.0)).max(h);
    let fill = c.rect(x, y, fw, h);
    c.p.rect_filled(fill, Rounding::same(c.px(h * 0.5)), c.ink(color, 1.0));
    c.p.rect_filled(
        Rect::from_min_size(fill.min, Vec2::new(fill.width(), fill.height() * 0.45)),
        Rounding::same(c.px(h * 0.25)),
        c.ink(Color32::WHITE, 0.16),
    );
}

// ------------------------------------------------------------------ iconos
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ic {
    Home,
    Garage,
    Wrench,
    Chart,
    User,
    Fuel,
    Doc,
    Plus,
    Check,
    Back,
    Bell,
    Gear,
    Chevron,
    Calendar,
    Photo,
    Bolt,
    Gauge,
    Bike,
    Trash,
    Pencil,
    Mic,
}

fn icon(c: &Canvas, ic: Ic, center: Pos2, size: f32, color: Color32) {
    let p = |nx: f32, ny: f32| Pos2::new(center.x + (nx - 0.5) * size, center.y + (ny - 0.5) * size);
    let w = (size * 0.085).max(1.0);
    let st = Stroke::new(w, color);
    let line = |pts: Vec<Pos2>, s: Stroke| c.p.add(egui::Shape::line(pts, s));
    let closed = |pts: Vec<Pos2>, s: Stroke| c.p.add(egui::Shape::closed_line(pts, s));
    let dot = |x: f32, y: f32, r: f32| c.p.circle_filled(p(x, y), size * r, color);

    match ic {
        Ic::Home => {
            line(vec![p(0.16, 0.50), p(0.50, 0.18), p(0.84, 0.50)], st);
            line(vec![p(0.26, 0.46), p(0.26, 0.84), p(0.74, 0.84), p(0.74, 0.46)], st);
        }
        Ic::Garage => {
            line(vec![p(0.12, 0.42), p(0.50, 0.16), p(0.88, 0.42)], st);
            line(vec![p(0.18, 0.42), p(0.18, 0.86), p(0.82, 0.86), p(0.82, 0.42)], st);
            line(vec![p(0.30, 0.86), p(0.30, 0.58), p(0.70, 0.58), p(0.70, 0.86)], st);
        }
        Ic::Wrench => {
            line(vec![p(0.28, 0.84), p(0.66, 0.44)], st);
            c.p.circle_stroke(p(0.72, 0.31), size * 0.17, st);
            line(vec![p(0.84, 0.19), p(0.76, 0.27)], st);
        }
        Ic::Chart => {
            line(vec![p(0.18, 0.84), p(0.84, 0.84)], Stroke::new(w * 0.9, color));
            line(vec![p(0.30, 0.84), p(0.30, 0.56)], st);
            line(vec![p(0.50, 0.84), p(0.50, 0.34)], st);
            line(vec![p(0.70, 0.84), p(0.70, 0.46)], st);
        }
        Ic::User => {
            c.p.circle_stroke(p(0.50, 0.34), size * 0.16, st);
            let mut pts = Vec::new();
            for i in 0..13 {
                let ang = std::f32::consts::PI * (0.08 + 0.84 * i as f32 / 12.0);
                pts.push(Pos2::new(
                    center.x - (size * 0.30) * ang.cos(),
                    center.y + (size * 0.24) * ang.sin() + size * 0.28,
                ));
            }
            line(pts, st);
        }
        Ic::Fuel => {
            line(vec![p(0.24, 0.84), p(0.24, 0.26), p(0.56, 0.26), p(0.56, 0.84)], st);
            line(vec![p(0.24, 0.84), p(0.56, 0.84)], st);
            line(vec![p(0.56, 0.38), p(0.70, 0.38), p(0.70, 0.68), p(0.80, 0.68)], st);
            c.p.rect_filled(
                Rect::from_min_max(p(0.30, 0.34), p(0.50, 0.48)),
                Rounding::same(c.px(1.5)),
                color,
            );
        }
        Ic::Doc => {
            closed(
                vec![p(0.28, 0.16), p(0.62, 0.16), p(0.74, 0.30), p(0.74, 0.84), p(0.28, 0.84)],
                st,
            );
            line(vec![p(0.62, 0.16), p(0.62, 0.30), p(0.74, 0.30)], st);
            line(vec![p(0.36, 0.52), p(0.66, 0.52)], Stroke::new(w * 0.8, color));
            line(vec![p(0.36, 0.66), p(0.58, 0.66)], Stroke::new(w * 0.8, color));
        }
        Ic::Plus => {
            line(vec![p(0.50, 0.26), p(0.50, 0.74)], st);
            line(vec![p(0.26, 0.50), p(0.74, 0.50)], st);
        }
        Ic::Check => {
            line(vec![p(0.26, 0.52), p(0.44, 0.70), p(0.76, 0.32)], st);
        }
        Ic::Back => {
            line(vec![p(0.60, 0.24), p(0.34, 0.50), p(0.60, 0.76)], st);
        }
        Ic::Bell => {
            line(
                vec![
                    p(0.28, 0.66),
                    p(0.32, 0.56),
                    p(0.32, 0.42),
                    p(0.40, 0.28),
                    p(0.50, 0.24),
                    p(0.60, 0.28),
                    p(0.68, 0.42),
                    p(0.68, 0.56),
                    p(0.72, 0.66),
                ],
                st,
            );
            line(vec![p(0.26, 0.66), p(0.74, 0.66)], Stroke::new(w * 0.9, color));
            dot(0.50, 0.78, 0.05);
        }
        Ic::Gear => {
            c.p.circle_stroke(center, size * 0.17, st);
            for k in 0..8 {
                let ang = k as f32 * std::f32::consts::TAU / 8.0;
                let (sn, cs) = ang.sin_cos();
                line(
                    vec![
                        Pos2::new(center.x + cs * size * 0.24, center.y + sn * size * 0.24),
                        Pos2::new(center.x + cs * size * 0.37, center.y + sn * size * 0.37),
                    ],
                    Stroke::new(w * 0.9, color),
                );
            }
        }
        Ic::Chevron => {
            line(vec![p(0.40, 0.26), p(0.64, 0.50), p(0.40, 0.74)], st);
        }
        Ic::Calendar => {
            closed(
                vec![p(0.20, 0.30), p(0.80, 0.30), p(0.80, 0.84), p(0.20, 0.84)],
                Stroke::new(w * 0.9, color),
            );
            line(vec![p(0.20, 0.46), p(0.80, 0.46)], Stroke::new(w * 0.8, color));
            line(vec![p(0.34, 0.22), p(0.34, 0.36)], st);
            line(vec![p(0.66, 0.22), p(0.66, 0.36)], st);
        }
        Ic::Photo => {
            closed(
                vec![p(0.18, 0.26), p(0.82, 0.26), p(0.82, 0.76), p(0.18, 0.76)],
                Stroke::new(w * 0.9, color),
            );
            c.p.circle_filled(p(0.36, 0.40), size * 0.06, color);
            line(
                vec![p(0.22, 0.72), p(0.44, 0.48), p(0.60, 0.64), p(0.70, 0.54), p(0.80, 0.72)],
                Stroke::new(w * 0.9, color),
            );
        }
        Ic::Bolt => {
            closed(
                vec![p(0.56, 0.14), p(0.32, 0.54), p(0.48, 0.54), p(0.42, 0.86), p(0.68, 0.44), p(0.52, 0.44)],
                Stroke::new(w * 0.9, color),
            );
        }
        Ic::Gauge => {
            let mut pts = Vec::new();
            for i in 0..17 {
                let ang = std::f32::consts::PI * (1.0 + i as f32 / 16.0);
                pts.push(Pos2::new(
                    center.x + (size * 0.34) * ang.cos(),
                    center.y + (size * 0.30) * ang.sin(),
                ));
            }
            line(pts, Stroke::new(w * 0.9, color));
            line(vec![center, Pos2::new(center.x + size * 0.20, center.y - size * 0.18)], st);
        }
        Ic::Bike => {
            c.p.circle_stroke(p(0.26, 0.66), size * 0.16, Stroke::new(w * 0.9, color));
            c.p.circle_stroke(p(0.74, 0.66), size * 0.16, Stroke::new(w * 0.9, color));
            line(vec![p(0.26, 0.66), p(0.46, 0.46), p(0.64, 0.46), p(0.74, 0.66)], Stroke::new(w * 0.9, color));
            line(vec![p(0.46, 0.46), p(0.56, 0.66)], Stroke::new(w * 0.8, color));
            line(vec![p(0.60, 0.34), p(0.70, 0.34)], Stroke::new(w * 0.9, color));
        }
        Ic::Trash => {
            line(vec![p(0.24, 0.30), p(0.76, 0.30)], st);
            line(vec![p(0.34, 0.30), p(0.36, 0.20), p(0.64, 0.20), p(0.66, 0.30)], Stroke::new(w * 0.9, color));
            closed(
                vec![p(0.30, 0.30), p(0.34, 0.84), p(0.66, 0.84), p(0.70, 0.30)],
                Stroke::new(w * 0.9, color),
            );
            line(vec![p(0.44, 0.42), p(0.44, 0.72)], Stroke::new(w * 0.7, color));
            line(vec![p(0.56, 0.42), p(0.56, 0.72)], Stroke::new(w * 0.7, color));
        }
        Ic::Pencil => {
            line(vec![p(0.24, 0.76), p(0.30, 0.58), p(0.66, 0.22), p(0.78, 0.34), p(0.42, 0.70)], st);
            line(vec![p(0.24, 0.76), p(0.42, 0.70)], Stroke::new(w * 0.8, color));
        }
        Ic::Mic => {
            // cápsula del micrófono
            c.p.rect_stroke(
                Rect::from_min_max(p(0.38, 0.16), p(0.62, 0.56)),
                Rounding::same(size * 0.12),
                st,
            );
            // arco que sostiene la cápsula
            let mut arco = Vec::new();
            for i in 0..=12 {
                let ang = std::f32::consts::PI * (i as f32 / 12.0);
                arco.push(Pos2::new(
                    center.x + (size * 0.22) * ang.cos(),
                    center.y + (size * 0.16) + (size * 0.20) * ang.sin(),
                ));
            }
            line(arco, Stroke::new(w * 0.9, color));
            // pie
            line(vec![p(0.50, 0.72), p(0.50, 0.84)], Stroke::new(w * 0.9, color));
            line(vec![p(0.38, 0.84), p(0.62, 0.84)], Stroke::new(w * 0.9, color));
        }
    }
}

fn icono_circulo(c: &Canvas, ic: Ic, centro: Pos2, radio: f32, bg: Color32, fg: Color32) {
    c.p.circle_filled(centro, radio, bg);
    icon(c, ic, centro, radio * 0.95, fg);
}

/// Botón dibujado a mano. Devuelve true si lo tocaron.
fn boton(c: &Canvas, ui: &mut egui::Ui, r: Rect, id: &str, label: &str, primario: bool) -> bool {
    let resp = ui.interact(r, Id::new(id), Sense::click());
    let pressed = resp.is_pointer_button_down_on();
    if primario {
        c.p.rect_filled(
            r,
            Rounding::same(c.px(14.0)),
            c.ink(if pressed { ACCENT_DEEP } else { ACCENT }, 1.0),
        );
        c.p.rect_stroke(
            r,
            Rounding::same(c.px(14.0)),
            Stroke::new(1.0, c.ink(Color32::WHITE, if pressed { 0.1 } else { 0.22 })),
        );
        txt_sc(c, r.center(), label, 10.6, c.ink(Color32::WHITE, 1.0), SEMI);
    } else {
        c.p.rect_filled(r, Rounding::same(c.px(14.0)), c.ink(if pressed { SURF3 } else { SURF2 }, 1.0));
        c.p.rect_stroke(r, Rounding::same(c.px(14.0)), Stroke::new(1.0, c.ink(LINE, 1.0)));
        txt_sc(c, r.center(), label, 10.6, c.ink(TEXT, 0.95), SEMI);
    }
    resp.clicked()
}

// ------------------------------------------------------------------ estado
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Inicio,
    Garaje,
    Manto,
    Stats,
    Perfil,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormKind {
    VehiculoNuevo,
    VehiculoEditar(usize),
    Tanqueo,
    ServicioNuevo,
    ServicioEditar(usize),
    Documento,
    DocumentoEditar(usize),
    Perfil,
    Precio,
}

#[derive(Clone, PartialEq, Eq)]
enum Pantalla {
    Onboarding,
    Principal,
    Detalle(usize),
    Form(FormKind),
    Caja,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Num {
    Entero,
    Decimal,
}

/// Borradores de texto de los formularios. Se rellenan al abrir cada uno.
#[derive(Default, Clone)]
struct Campos {
    marca: String,
    modelo: String,
    year: String,
    km: String,
    placa: String,
    color: String,
    cc: String,

    fecha: String,
    estacion: String,
    monto: String,
    precio_galon: String,

    s_nombre: String,
    s_detalle: String,
    s_cada: String,
    s_ultimo: String,
    s_costo: String,
    s_fecha: String,

    d_nombre: String,
    d_vence: String,

    /// Texto de la caja inteligente.
    caja: String,

    p_nombre: String,
    p_email: String,
    p_ciudad: String,
    p_telefono: String,
}

impl Campos {
    /// Devuelve el texto editable del campo con ese id (lo usa el teclado).
    fn buf_mut(&mut self, id: &str) -> Option<&mut String> {
        Some(match id {
            "v_marca" => &mut self.marca,
            "v_modelo" => &mut self.modelo,
            "v_year" => &mut self.year,
            "v_km" => &mut self.km,
            "v_placa" => &mut self.placa,
            "v_color" => &mut self.color,
            "v_cc" => &mut self.cc,
            "t_fecha" => &mut self.fecha,
            "t_km" => &mut self.km,
            "t_monto" => &mut self.monto,
            "t_precio" => &mut self.precio_galon,
            "t_est" => &mut self.estacion,
            "s_nombre" => &mut self.s_nombre,
            "s_detalle" => &mut self.s_detalle,
            "s_cada" => &mut self.s_cada,
            "s_ult" => &mut self.s_ultimo,
            "s_costo" => &mut self.s_costo,
            "s_fecha" => &mut self.s_fecha,
            "d_nombre" => &mut self.d_nombre,
            "d_vence" => &mut self.d_vence,
            "caja" => &mut self.caja,
            "p_nombre" => &mut self.p_nombre,
            "p_email" => &mut self.p_email,
            "p_ciudad" => &mut self.p_ciudad,
            "p_tel" => &mut self.p_telefono,
            "pr_gal" => &mut self.precio_galon,
            _ => return None,
        })
    }
}

struct Toast {
    texto: String,
    at: f64,
}

// ------------------------------------------------------------- campos de texto
fn etiqueta(c: &Canvas, x: f32, y: f32, s: &str) {
    txt(c, x, y, s, 8.4, c.ink(MUTED, 1.0), SEMI);
}

/// Dibuja un campo de texto: fondo, borde, contenido y cursor.
fn pintar_campo(c: &Canvas, ui: &egui::Ui, r: Rect, buf: &str, activo: bool, hint: &str) {
    let radio = Rounding::same(c.px(12.0));
    c.p.rect_filled(r, radio, c.ink(if activo { SURF2 } else { SURF }, 1.0));
    c.p.rect_stroke(
        r,
        radio,
        Stroke::new(
            c.px(if activo { 1.6 } else { 1.0 }),
            c.ink(
                if activo { ACCENT } else { LINE },
                if activo { 0.9 } else { 1.0 },
            ),
        ),
    );

    let pad = c.px(12.0);
    let inner = Rect::from_min_max(
        Pos2::new(r.left() + pad, r.top() + c.px(2.0)),
        Pos2::new(r.right() - pad, r.bottom() - c.px(2.0)),
    );
    let fuente = FontId::new(c.px(13.0), fam(REG));

    if buf.is_empty() {
        if !activo {
            c.p.text(
                Pos2::new(inner.left(), inner.center().y),
                Align2::LEFT_CENTER,
                hint,
                fuente.clone(),
                c.ink(MUTED, 0.8),
            );
            return;
        }
    } else {
        let gal = ui
            .painter()
            .layout_no_wrap(buf.to_string(), fuente.clone(), c.ink(TEXT, 1.0));
        let tw = gal.size().x;
        let dx = (tw - inner.width()).max(0.0);
        let pos = Pos2::new(inner.left() - dx, inner.center().y - gal.size().y * 0.5);
        let clip = ui.painter().with_clip_rect(inner);
        clip.galley(pos, gal, c.ink(TEXT, 1.0));
    }

    if activo {
        let t = ui.input(|i| i.time);
        if (t * 1.6) as i64 % 2 == 0 {
            let gal = ui
                .painter()
                .layout_no_wrap(buf.to_string(), fuente, c.ink(TEXT, 1.0));
            let cx = inner.left() + gal.size().x.min(inner.width()) + c.px(1.0);
            c.p.line_segment(
                [
                    Pos2::new(cx, inner.top() + c.px(4.0)),
                    Pos2::new(cx, inner.bottom() - c.px(4.0)),
                ],
                Stroke::new(c.px(1.4), c.ink(ACCENT, 1.0)),
            );
        }
    }
}

fn campo_txt(
    c: &Canvas,
    ui: &mut egui::Ui,
    r: Rect,
    id: &str,
    buf: &mut String,
    foco: &mut Option<String>,
    hint: &str,
) {
    let activo = foco.as_deref() == Some(id);
    let resp = ui.interact(r, Id::new((id, "campo")), Sense::click());
    pintar_campo(c, ui, r, buf, activo, hint);
    if resp.clicked() {
        CLICS_CAMPO.fetch_add(1, Ordering::Relaxed);
        *foco = Some(id.to_string());
    }
}

/// Campo numérico con botones − y + para ajustar rápido con el dedo.
fn campo_num(
    c: &Canvas,
    ui: &mut egui::Ui,
    r: Rect,
    id: &str,
    buf: &mut String,
    foco: &mut Option<String>,
    hint: &str,
    paso: f64,
    min: f64,
    max: f64,
    tipo: Num,
) {
    let bw = c.px(42.0);
    let gap = c.px(8.0);
    let r_txt = Rect::from_min_max(
        r.min,
        Pos2::new(r.right() - (2.0 * bw + 2.0 * gap), r.bottom()),
    );
    campo_txt(c, ui, r_txt, id, buf, foco, hint);

    let r_menos = Rect::from_min_size(
        Pos2::new(r_txt.right() + gap, r.top()),
        Vec2::new(bw, r.height()),
    );
    let r_mas = Rect::from_min_size(
        Pos2::new(r_menos.right() + gap, r.top()),
        Vec2::new(bw, r.height()),
    );
    for (rr, signo) in [(r_menos, -1.0f64), (r_mas, 1.0f64)] {
        let resp = ui.interact(rr, Id::new((id, "st", signo as i32)), Sense::click());
        card(
            c,
            rr,
            12.0,
            c.ink(
                if resp.is_pointer_button_down_on() {
                    SURF3
                } else {
                    SURF2
                },
                1.0,
            ),
            1.0,
        );
        let cen = rr.center();
        let w = c.px(7.0);
        c.p.line_segment(
            [Pos2::new(cen.x - w, cen.y), Pos2::new(cen.x + w, cen.y)],
            Stroke::new(1.6, c.ink(TEXT, 0.9)),
        );
        if signo > 0.0 {
            c.p.line_segment(
                [Pos2::new(cen.x, cen.y - w), Pos2::new(cen.x, cen.y + w)],
                Stroke::new(1.6, c.ink(TEXT, 0.9)),
            );
        }
        if resp.clicked() {
            let v = match tipo {
                Num::Entero => solo_digitos(buf) as f64,
                Num::Decimal => a_dec(buf),
            };
            let nv = (v + signo * paso).clamp(min, max);
            *buf = match tipo {
                Num::Entero => fmt_int(nv.round() as i64),
                Num::Decimal => fmt_dec(nv),
            };
        }
    }
}

/// ¿El campo se llena sobre todo con dígitos? Entonces abrimos la capa numérica.
fn es_numerico(id: &str) -> bool {
    matches!(
        id,
        "v_year"
            | "v_km"
            | "v_cc"
            | "t_km"
            | "t_monto"
            | "t_precio"
            | "t_fecha"
            | "s_cada"
            | "s_ult"
            | "s_costo"
            | "s_fecha"
            | "d_vence"
            | "p_tel"
            | "pr_gal"
    )
}

/// Campos que guardan una fecha. Se escriben solo con dígitos y las barras se
/// ponen solas (`15032027` → `15/03/2027`), así no hace falta la tecla `/`.
fn es_fecha(id: &str) -> bool {
    matches!(id, "t_fecha" | "s_fecha" | "d_vence")
}

/// Da formato `dd/mm/aaaa` a una tira de dígitos (hasta 8).
fn formatear_fecha(digitos: &str) -> String {
    let mut out = String::new();
    for (i, ch) in digitos.chars().take(8).enumerate() {
        if i == 2 || i == 4 {
            out.push('/');
        }
        out.push(ch);
    }
    out
}

/// Toques a campos de texto: sirve para reabrir el teclado del sistema aunque
/// el campo enfocado no cambie (el IME se descarta con el gesto "atrás").
static CLICS_CAMPO: AtomicU64 = AtomicU64::new(0);

/// Qué teclado del sistema conviene para un campo (solo Android).
#[cfg(target_os = "android")]
fn teclado_para(id: &str) -> ime::Teclado {
    use ime::Teclado;
    match id {
        "v_year" | "v_km" | "v_cc" | "t_km" | "t_monto" | "t_precio" | "s_cada" | "s_ult"
        | "s_costo" | "p_tel" | "pr_gal" | "t_fecha" | "s_fecha" | "d_vence" => Teclado::Numero,
        _ => Teclado::Texto,
    }
}

/// Texto que muestra una tecla según el estado de mayúsculas.
fn etiqueta_tecla(t: Tecla, mayus: bool) -> String {
    match t {
        Tecla::Letra(ch) => {
            if mayus {
                ch.to_uppercase().to_string()
            } else {
                ch.to_string()
            }
        }
        Tecla::Simbolo(s) => s.to_string(),
        Tecla::Mayus => "Mayús".to_string(),
        Tecla::Borrar => "Borrar".to_string(),
        Tecla::Capa(Capa::Letras) => "ABC".to_string(),
        Tecla::Capa(Capa::Numeros) => "123".to_string(),
        Tecla::Espacio => "espacio".to_string(),
    }
}

/// Dibuja una tecla del teclado en pantalla y dice si la tocaron.
fn tecla_kb(
    c: &Canvas,
    ui: &mut egui::Ui,
    r: Rect,
    label: &str,
    id: &str,
    util: bool,
    activa: bool,
    size: f32,
) -> bool {
    let resp = ui.interact(r, Id::new(id), Sense::click());
    let pulsada = resp.is_pointer_button_down_on();
    let radio = Rounding::same(c.px(10.0));
    let fondo = if activa {
        ACCENT_DEEP
    } else if pulsada {
        SURF3
    } else if util {
        SURF3
    } else {
        SURF
    };
    c.p.rect_filled(r, radio, c.ink(fondo, 1.0));
    if activa {
        c.p.rect_stroke(r, radio, Stroke::new(c.px(1.4), c.ink(ACCENT, 1.0)));
    }
    let color = if activa { ACCENT } else { TEXT };
    txt_sc(c, r.center(), label, size, c.ink(color, 0.95), MED);
    resp.clicked()
}

/// Zona desplazable: devuelve un lienzo desplazado y recortado.
fn scrollable(
    c: &Canvas,
    ui: &mut egui::Ui,
    scroll: &mut f32,
    contenido: f32,
    top: f32,
    bottom: f32,
) -> Canvas {
    let max = (contenido - (bottom - top)).max(0.0);
    let wheel = ui.input(|i| i.raw_scroll_delta.y);
    if wheel != 0.0 {
        *scroll = (*scroll - wheel / c.s).clamp(0.0, max);
    }
    let pointer = ui.ctx().input(|i| i.pointer.interact_pos());
    let pointer_down = ui.ctx().input(|i| i.pointer.primary_down());
    let drag = ui.ctx().input(|i| i.pointer.delta().y);
    if pointer_down {
        if let Some(pos) = pointer {
            if c.rect(0.0, top, c.cw, bottom - top).contains(pos) {
                if drag.abs() > 0.01 {
                    *scroll = (*scroll - drag / c.s).clamp(0.0, max);
                }
            }
        }
    }
    *scroll = scroll.clamp(0.0, max);
    let mut inner = c.with(1.0, -(*scroll));
    inner.p = c.p.with_clip_rect(c.rect(0.0, top, c.cw, bottom - top));
    inner
}

// --------------------------------------------------------------- geometría
#[derive(Clone, Copy, PartialEq, Eq)]
enum Modo {
    Telefono,
    Tablet,
}

/// Geometría del lienzo. En teléfono reproduce el diseño compacto de 400×866
/// con barra inferior; en tablet usa una barra de navegación lateral y varias
/// columnas de contenido para aprovechar el ancho de la pantalla.
#[derive(Clone, Copy)]
struct Lay {
    modo: Modo,
    /// Ancho total del lienzo base (unidades de diseño).
    bw: f32,
    /// Alto total del lienzo base (unidades de diseño).
    bh: f32,
    /// Ancho de la barra lateral (0 en teléfono).
    rail: f32,
    /// Número de columnas de contenido disponibles (1..3).
    cols: usize,
    /// Ancho de una columna.
    cw: f32,
    /// Paso entre columnas (ancho + hueco).
    paso: f32,
    /// Recorte vertical del área con scroll.
    top: f32,
    bottom: f32,
}

impl Lay {
    fn telefono() -> Self {
        Lay {
            modo: Modo::Telefono,
            bw: BASE_W,
            bh: BASE_H,
            rail: 0.0,
            cols: 1,
            cw: BASE_W,
            paso: BASE_W,
            top: 82.0,
            bottom: 782.0,
        }
    }
}

// ------------------------------------------------------------------- app
struct App {
    clock: f64,
    frame: u64,
    datos: Datos,
    pantalla: Pantalla,
    volver: Pantalla,
    tab: Tab,
    entrada: f64,
    form: Campos,
    toast: Option<Toast>,
    confirmar: Option<(String, f64)>,
    scroll: f32,
    detalle_tab: usize,
    tab_anim: f32,
    /// Geometría del lienzo (se recalcula cada fotograma en `dibujar`).
    lay: Lay,
    foto: Option<egui::TextureHandle>,
    shot: Option<PathBuf>,
    /// Campo de texto con el foco (id del campo) y estado del teclado propio.
    foco: Option<String>,
    foco_prev: Option<String>,
    mayus: bool,
    capa: Capa,
    /// Último resultado que mostró la caja inteligente.
    caja_estado: String,
    /// ¿El micrófono está escuchando? (solo Android).
    escuchando: bool,
    /// Estado del teclado del sistema en Android (ver `ime_android`).
    #[cfg(target_os = "android")]
    clics_prev: u64,
    #[cfg(target_os = "android")]
    ime_abierto: bool,
}

/// Capas del teclado en pantalla.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Capa {
    Letras,
    Numeros,
}

/// Acción de una tecla del teclado en pantalla.
#[derive(Clone, Copy)]
enum Tecla {
    Letra(char),
    Simbolo(&'static str),
    Borrar,
    Mayus,
    Capa(Capa),
    Espacio,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, shot: Option<PathBuf>, datos: Datos) -> Self {
        setup_fonts(&cc.egui_ctx);
        let foto = load_photo(&cc.egui_ctx);
        let pantalla = if datos.vehiculos.is_empty() {
            Pantalla::Onboarding
        } else {
            Pantalla::Principal
        };
        App {
            clock: 0.0,
            frame: 0,
            datos,
            pantalla,
            volver: Pantalla::Principal,
            tab: Tab::Inicio,
            entrada: -1.0,
            form: Campos::default(),
            toast: None,
            confirmar: None,
            scroll: 0.0,
            detalle_tab: 0,
            tab_anim: 0.0,
            lay: Lay::telefono(),
            foto,
            shot,
            foco: None,
            foco_prev: None,
            mayus: false,
            capa: Capa::Letras,
            caja_estado: String::new(),
            escuchando: false,
            #[cfg(target_os = "android")]
            clics_prev: 0,
            #[cfg(target_os = "android")]
            ime_abierto: false,
        }
    }

    fn guardar(&mut self) {
        if let Err(e) = datos::guardar(&self.datos) {
            eprintln!("{e}");
            self.avisar(&e);
        }
    }

    fn avisar(&mut self, texto: &str) {
        self.toast = Some(Toast {
            texto: texto.to_string(),
            at: self.clock,
        });
    }

    fn ir(&mut self, p: Pantalla) {
        self.pantalla = p;
        self.entrada = self.clock;
        self.scroll = 0.0;
    }

    /// Confirmación de dos toques para acciones destructivas.
    fn confirmar(&mut self, clave: &str) -> bool {
        if let Some((k, t)) = &self.confirmar {
            if k == clave && self.clock - t < 3.0 {
                self.confirmar = None;
                return true;
            }
        }
        self.confirmar = Some((clave.to_string(), self.clock));
        false
    }

    fn confirmando(&self, clave: &str) -> bool {
        matches!(&self.confirmar, Some((k, t)) if k == clave && self.clock - t < 3.0)
    }

    fn abrir_form(&mut self, k: FormKind) {
        self.form = Campos::default();
        self.foco = None;
        self.foco_prev = None;
        self.mayus = false;
        self.capa = Capa::Letras;
        let v = self.datos.indice();
        match k {
            FormKind::VehiculoNuevo => {
                self.form.year = format!("{}", datos::hoy().0);
                self.form.cc = "150".into();
                self.form.km = "0".into();
            }
            FormKind::VehiculoEditar(i) => {
                if let Some(ve) = self.datos.vehiculos.get(i) {
                    self.form.marca = ve.marca.clone();
                    self.form.modelo = ve.modelo.clone();
                    self.form.year = format!("{}", ve.year);
                    self.form.km = format!("{}", ve.km);
                    self.form.placa = ve.placa.clone();
                    self.form.color = ve.color.clone();
                    self.form.cc = format!("{}", ve.cc);
                }
            }
            FormKind::Tanqueo => {
                self.form.fecha = datos::hoy_txt();
                self.form.monto = String::new();
                self.form.precio_galon = if self.datos.precio_galon > 0.0 {
                    format!("{}", self.datos.precio_galon.round() as i64)
                } else {
                    String::new()
                };
                self.form.km = format!("{}", self.datos.km_actual());
            }
            FormKind::ServicioNuevo => {
                self.form.s_cada = "3000".into();
                self.form.s_ultimo = format!("{}", self.datos.km_actual());
            }
            FormKind::ServicioEditar(i) => {
                if let Some(s) = self.datos.servicios.get(i) {
                    self.form.s_nombre = s.nombre.clone();
                    self.form.s_detalle = s.detalle.clone();
                    self.form.s_cada = format!("{}", s.cada_km);
                    self.form.s_ultimo = format!("{}", s.ultimo_km);
                    self.form.s_costo = format!("{}", s.costo.round() as i64);
                    self.form.s_fecha = s.ultima_fecha.clone();
                }
            }
            FormKind::Documento => {
                self.form.d_vence = datos::fecha_hace_meses(0, 1);
            }
            FormKind::DocumentoEditar(i) => {
                if let Some(d) = self.datos.docs.get(i) {
                    self.form.d_nombre = d.nombre.clone();
                    self.form.d_vence = d.vence.clone();
                }
            }
            FormKind::Perfil => {
                self.form.p_nombre = self.datos.perfil.nombre.clone();
                self.form.p_email = self.datos.perfil.email.clone();
                self.form.p_ciudad = self.datos.perfil.ciudad.clone();
                self.form.p_telefono = self.datos.perfil.telefono.clone();
            }
            FormKind::Precio => {
                if self.datos.precio_galon > 0.0 {
                    self.form.precio_galon = format!("{}", self.datos.precio_galon.round() as i64);
                }
            }
        }
        let _ = v;
        self.volver = self.pantalla.clone();
        self.ir(Pantalla::Form(k));
    }

    fn cerrar_form(&mut self) {
        self.foco = None;
        self.foco_prev = None;
        self.mayus = false;
        let p = self.volver.clone();
        self.ir(p);
    }

    fn marcar_servicio(&mut self, i: usize) {
        let km = self.datos.km_actual();
        if let Some(s) = self.datos.servicios.get_mut(i) {
            s.ultimo_km = km;
            s.ultima_fecha = datos::hoy_txt();
        }
        self.guardar();
        self.avisar("Servicio registrado");
    }

    /// Abre la caja inteligente con el campo de texto vacío.
    fn abrir_caja(&mut self) {
        self.foco = None;
        self.foco_prev = None;
        self.mayus = false;
        self.capa = Capa::Letras;
        self.form.caja.clear();
        self.volver = self.pantalla.clone();
        self.ir(Pantalla::Caja);
    }

    /// Interpreta lo que se dijo o escribió y guarda directamente lo entendido.
    fn interpretar_caja(&mut self) {
        let texto = self.form.caja.trim().to_string();
        if texto.is_empty() {
            self.avisar("Dime o escribe algo");
            return;
        }
        let v = self.datos.indice();
        let km_actual = self.datos.km_actual();
        let hoy = datos::hoy_txt();
        let dia = datos::hoy().2;
        match entender::interpretar(&texto, datos::hoy()) {
            entender::Idea::Documento { nombre, vence } => {
                let vence = vence.unwrap_or_else(|| datos::fecha_mas_meses(12, dia));
                self.datos.docs.push(Documento {
                    vehiculo: v,
                    nombre: nombre.clone(),
                    vence: vence.clone(),
                });
                let msg = format!("Documento «{nombre}» guardado · vence {vence}");
                self.cerrar_caja_guardado(&msg);
            }
            entender::Idea::Servicio {
                nombre,
                cada_km,
                costo,
                ultimo_km,
                fecha,
            } => {
                self.datos.servicios.push(Servicio {
                    vehiculo: v,
                    nombre: nombre.clone(),
                    detalle: String::new(),
                    cada_km: cada_km.unwrap_or(3000).max(1),
                    ultimo_km: ultimo_km.unwrap_or(km_actual),
                    costo: costo.unwrap_or(0.0),
                    ultima_fecha: fecha.unwrap_or_else(|| hoy.clone()),
                });
                let msg = format!("Servicio «{nombre}» agregado");
                self.cerrar_caja_guardado(&msg);
            }
            entender::Idea::Tanqueo {
                monto,
                precio,
                km,
                estacion,
            } => match monto.filter(|m| *m > 0.0) {
                Some(m) => {
                    let precio = precio.unwrap_or(if self.datos.precio_galon > 0.0 {
                        self.datos.precio_galon
                    } else {
                        13_900.0
                    });
                    let kmv = km.unwrap_or(km_actual);
                    self.datos.tanqueos.push(Tanqueo {
                        vehiculo: v,
                        fecha: hoy.clone(),
                        estacion,
                        monto: m,
                        galones: 0.0,
                        precio_galon: precio,
                        km: kmv,
                    });
                    self.datos.precio_galon = precio;
                    if let Some(ve) = self.datos.vehiculos.get_mut(v) {
                        if kmv > ve.km {
                            ve.km = kmv;
                        }
                    }
                    let msg = format!("Tanqueo guardado · {}", money(m));
                    self.cerrar_caja_guardado(&msg);
                }
                None => self.avisar("No entendí el monto. Di por ejemplo «tanqueé 50.000»"),
            },
            entender::Idea::Nada => {
                self.avisar("No entendí. Prueba: «el SOAT vence 15/03/2027»");
            }
        }
    }

    /// Cierra el teclado tras un guardado de la caja y deja el aviso visible.
    fn cerrar_caja_guardado(&mut self, msg: &str) {
        self.form.caja.clear();
        self.foco = None;
        self.guardar();
        self.caja_estado = msg.to_string();
        self.avisar(msg);
    }

    fn borrar_todo(&mut self) {
        self.datos = Datos::default();
        datos::borrar();
        self.guardar();
        self.tab = Tab::Inicio;
        self.ir(Pantalla::Onboarding);
        self.avisar("Datos borrados");
    }
}

// ------------------------------------------------------------------ raíz
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame += 1;
        self.clock += 1.0 / 60.0;
        ctx.request_repaint();
        if self.entrada < 0.0 {
            self.entrada = self.clock;
        }

        // Android: si ya dibujamos un fotograma, el arranque fue bien. Borramos
        // el rastro de diagnóstico para que no vuelva a aparecer.
        #[cfg(target_os = "android")]
        if self.frame == 1 {
            ime::limpiar();
        }

        // Teclado del sistema (Android): vuelca lo escrito y abre/cierra el IME.
        #[cfg(target_os = "android")]
        self.ime_android();

        // captura de pantalla
        let mut shot: Option<(Vec<u8>, usize, usize)> = None;
        ctx.input(|i| {
            for ev in &i.events {
                if let egui::Event::Screenshot { image, .. } = ev {
                    let mut buf = Vec::with_capacity(image.pixels.len() * 4);
                    for px in &image.pixels {
                        buf.extend_from_slice(&px.to_array());
                    }
                    shot = Some((buf, image.size[0], image.size[1]));
                }
            }
        });
        if let (Some((buf, w, h)), Some(path)) = (shot, self.shot.clone()) {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            match image::save_buffer(&path, &buf, w as u32, h as u32, image::ColorType::Rgba8) {
                Ok(()) => println!("captura guardada en {}", path.display()),
                Err(e) => eprintln!("no pude guardar: {e}"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(BG))
            .show(ctx, |ui| {
                self.dibujar(ui);
            });

        if self.shot.is_some() {
            if self.frame == 60 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
            if self.frame > 900 {
                eprintln!("no llegó la captura");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl App {
    fn dibujar(&mut self, ui: &mut egui::Ui) {
        // En Android la ventana es "edge-to-edge": el sistema dibuja la barra de
        // estado y la de navegación ENCIMA del contenido. Recortamos el lienzo
        // para que la barra inferior de la app no quede debajo de los botones
        // táctiles. Los insets llegan en píxeles físicos; egui trabaja en puntos.
        #[allow(unused_mut)]
        let mut avail = ui.max_rect();
        #[cfg(target_os = "android")]
        {
            let ppp = ui.ctx().pixels_per_point().max(0.01);
            let (arriba_px, abajo_px) = ime::insets();
            let alto = avail.height();
            let arriba = (arriba_px as f32 / ppp).clamp(0.0, alto * 0.2);
            let abajo = (abajo_px as f32 / ppp).clamp(0.0, alto * 0.2);
            avail.min.y += arriba;
            avail.max.y -= abajo;
        }
        // ¿teléfono compacto o tablet? El tamaño se mide en puntos (≈ dp).
        let min_lado = avail.width().min(avail.height());
        let tablet = min_lado >= 600.0;

        let (s, bw, bh, o);
        if tablet {
            let rail = 84.0;
            let cw = 400.0;
            let hueco = 28.0;
            let margen = 24.0;
            // ¿Caben dos columnas? Solo cuando la pantalla es claramente
            // apaisada y hay ancho de sobra. En vertical se usa una sola
            // columna centrada.
            let dos = avail.width() / avail.height() >= 1.15 && avail.width() >= 900.0;
            let cols = if dos { 2 } else { 1 };
            // Los tableros (Inicio y Estadísticas) reparten su contenido en
            // dos columnas; por eso necesitan un lienzo mucho más bajo que el
            // diseño de teléfono (866). El resto de pantallas usa una columna
            // y conserva el alto de diseño.
            let tablero = matches!(self.pantalla, Pantalla::Principal)
                && matches!(self.tab, Tab::Inicio | Tab::Stats);
            bh = if tablero && cols == 2 { 430.0 } else { BASE_H };
            // Escala: la que hace caber las columnas a lo ancho, sin pasarse
            // de alto. Si sobra alto, el lienzo se centra verticalmente.
            let ancho_necesario =
                rail + cols as f32 * cw + (cols as f32 - 1.0) * hueco + 2.0 * margen;
            s = (avail.width() / ancho_necesario)
                .min(avail.height() / bh)
                .max(0.01);
            bw = avail.width() / s;
            o = Pos2::new(
                avail.left(),
                avail.top() + (avail.height() - bh * s).max(0.0) * 0.5,
            );
            self.lay = Lay {
                modo: Modo::Tablet,
                bw,
                bh,
                rail,
                cols,
                cw,
                paso: cw + hueco,
                top: 82.0,
                bottom: bh - 12.0,
            };
        } else {
            s = (avail.width() / BASE_W).clamp(0.01, 2.0);
            bw = BASE_W;
            bh = (avail.height() / s).clamp(BASE_H * 0.92, BASE_H * 1.20);
            o = Pos2::new(
                avail.left() + (avail.width() - BASE_W * s) * 0.5,
                avail.top() + (avail.height() - bh * s).max(0.0) * 0.5,
            );
            let mut lay = Lay::telefono();
            lay.bh = bh;
            lay.bottom = bh - 84.0;
            self.lay = lay;
        }
        let c = Canvas {
            p: ui.painter().clone(),
            o,
            s,
            fade: 1.0,
            dy: 0.0,
            cw: self.lay.cw,
            paso: self.lay.paso,
        };

        c.p.rect_filled(avail, Rounding::ZERO, BG);
        let full = c.rect(0.0, 0.0, bw, bh);
        grad_v(&c, full, Color32::from_rgb(0x0E, 0x11, 0x15), BG);

        // línea de carga al cambiar de pantalla
        let carga = ((self.clock - self.entrada) / 0.55) as f32;
        if carga < 1.0 {
            let w = bw * ease_out(carga);
            c.p.rect_filled(
                c.rect(0.0, 0.0, w, 2.0),
                Rounding::ZERO,
                c.ink(ACCENT, 1.0 - ease_out(carga) * 0.9),
            );
        }

        match self.pantalla.clone() {
            Pantalla::Onboarding => {
                let cc = self.canvas_cols(&c, 1);
                self.onboarding(&cc, ui);
            }
            Pantalla::Principal => {
                if self.lay.modo == Modo::Tablet {
                    self.barra_lateral(&c, ui);
                } else {
                    self.barra_nav(&c, ui);
                }
                let tab = self.tab;
                match tab {
                    Tab::Inicio => {
                        let n = self.lay.cols;
                        let cc = self.canvas_cols(&c, n);
                        self.inicio(&cc, ui);
                    }
                    Tab::Stats => {
                        let n = self.lay.cols;
                        let cc = self.canvas_cols(&c, n);
                        self.estadisticas(&cc, ui);
                    }
                    Tab::Garaje => {
                        let cc = self.canvas_cols(&c, 1);
                        self.garaje(&cc, ui);
                    }
                    Tab::Manto => {
                        let cc = self.canvas_cols(&c, 1);
                        self.mantenimiento(&cc, ui);
                    }
                    Tab::Perfil => {
                        let cc = self.canvas_cols(&c, 1);
                        self.perfil(&cc, ui);
                    }
                }
            }
            Pantalla::Detalle(i) => {
                let cc = self.canvas_cols(&c, 1);
                self.detalle(&cc, ui, i);
            }
            Pantalla::Form(k) => {
                let cc = self.canvas_cols(&c, 1);
                self.formulario(&cc, ui, k);
            }
            Pantalla::Caja => {
                let cc = self.canvas_cols(&c, 1);
                self.caja(&cc, ui);
            }
        }
        self.toast_ui(&c);
    }

    /// Sub-lienzo del bloque de `n` columnas, centrado en el área de contenido.
    fn canvas_cols(&self, c: &Canvas, n: usize) -> Canvas {
        let l = &self.lay;
        let hueco = l.paso - l.cw;
        let total = n as f32 * l.cw + n.saturating_sub(1) as f32 * hueco;
        let area = l.bw - l.rail;
        let x = l.rail + (area - total).max(0.0) * 0.5;
        let mut cc = c.clone();
        cc.o.x = c.o.x + x * c.s;
        cc
    }

    // ======================================================== barra lateral
    /// Barra de navegación lateral (modo tablet).
    fn barra_lateral(&mut self, c: &Canvas, ui: &mut egui::Ui) {
        let l = self.lay;
        // La barra cubre toda la pantalla, no solo el lienzo de contenido
        // (que en los tableros a dos columnas es más bajo que la pantalla).
        let h0 = -400.0;
        let h1 = l.bh + 400.0;
        c.p.rect_filled(
            c.rect(0.0, h0, l.rail, h1 - h0),
            Rounding::ZERO,
            c.ink(SURF, 0.99),
        );
        c.p.line_segment(
            [c.pos(l.rail, h0), c.pos(l.rail, h1)],
            Stroke::new(1.0, c.ink(LINE, 1.0)),
        );

        let cx = l.rail * 0.5;
        c.p.circle_filled(c.pos(cx, 46.0), c.px(20.0), c.ink(ACCENT, 0.14));
        icon(c, Ic::Bike, c.pos(cx, 46.0), c.px(23.0), c.ink(ACCENT, 0.95));

        let items = [
            (Tab::Inicio, Ic::Home, "Inicio"),
            (Tab::Garaje, Ic::Garage, "Garaje"),
            (Tab::Manto, Ic::Wrench, "Servicio"),
            (Tab::Stats, Ic::Chart, "Datos"),
            (Tab::Perfil, Ic::User, "Perfil"),
        ];
        for (i, (tab, ic, label)) in items.iter().enumerate() {
            let cy = 118.0 + i as f32 * 86.0;
            let hit = c.rect(6.0, cy - 32.0, l.rail - 12.0, 78.0);
            let resp = ui.interact(hit, Id::new(("rail", i)), Sense::click());
            let activo = self.tab == *tab;
            if activo {
                c.p.rect_filled(hit, Rounding::same(c.px(16.0)), c.ink(ACCENT, 0.12));
                c.p.rect_filled(
                    c.rect(0.0, cy - 15.0, 3.0, 30.0),
                    Rounding::same(c.px(2.0)),
                    c.ink(ACCENT, 1.0),
                );
            }
            let col = if activo { ACCENT } else { MUTED };
            icon(
                c,
                *ic,
                c.pos(cx, cy - 8.0),
                c.px(20.0),
                c.ink(col, if activo { 1.0 } else { 0.82 }),
            );
            txt_c(
                c,
                cx,
                cy + 20.0,
                label,
                8.0,
                c.ink(if activo { TEXT } else { MUTED }, 1.0),
                if activo { SEMI } else { MED },
            );
            if resp.clicked() && !activo {
                self.tab = *tab;
                self.entrada = self.clock;
                self.scroll = 0.0;
            }
        }
    }

    // ============================================================ navegación
    fn barra_nav(&mut self, c: &Canvas, ui: &mut egui::Ui) {
        let top = self.lay.bh - 84.0;
        let r = c.rect(0.0, top, BASE_W, 84.0);
        c.p.rect_filled(r, Rounding::ZERO, c.ink(SURF, 0.98));
        divider(c, 0.0, BASE_W, top, 1.0);

        let items = [
            (Tab::Inicio, Ic::Home, "Inicio"),
            (Tab::Garaje, Ic::Garage, "Mi garaje"),
            (Tab::Manto, Ic::Wrench, "Mantenimiento"),
            (Tab::Stats, Ic::Chart, "Estadísticas"),
            (Tab::Perfil, Ic::User, "Perfil"),
        ];
        for (i, (tab, ic, label)) in items.iter().enumerate() {
            let cx = 40.0 + i as f32 * 80.0;
            let hit = c.rect(cx - 38.0, top + 6.0, 76.0, 68.0);
            let resp = ui.interact(hit, Id::new(("nav", i)), Sense::click());
            let activo = self.tab == *tab;
            let col = if activo { ACCENT } else { MUTED };

            if activo {
                let pill = c.rect(cx - 20.0, top + 10.0, 40.0, 28.0);
                c.p.rect_filled(pill, Rounding::same(c.px(14.0)), c.ink(ACCENT, 0.14));
            }
            icon(c, *ic, c.pos(cx, top + 24.0), c.px(19.0), c.ink(col, if activo { 1.0 } else { 0.85 }));
            txt_c(
                c,
                cx,
                top + 48.0,
                label,
                8.4,
                c.ink(if activo { TEXT } else { MUTED }, 1.0),
                if activo { SEMI } else { MED },
            );
            if resp.clicked() && !activo {
                self.tab = *tab;
                self.entrada = self.clock;
                self.scroll = 0.0;
            }
        }
    }
}

fn fecha_larga() -> String {
    let (_, m, d) = datos::hoy();
    format!("{}, {} de {}", datos::semana(), d, datos::mes_nombre(m))
}

impl App {
    // ============================================================ onboarding
    fn onboarding(&mut self, base: &Canvas, ui: &mut egui::Ui) {
        let e = ease_out(((self.clock - self.entrada) / 0.5) as f32);
        let c = &base.with(e, (1.0 - e) * 12.0);

        txt_c(c, 200.0, 42.0, "MI GARAJE", 20.0, c.ink(TEXT, 1.0), BOLD);
        txt_c(c, 200.0, 66.0, "Tu garaje digital · empieza de cero", 9.6, c.ink(MUTED, 1.0), MED);
        icon(c, Ic::Bike, c.pos(200.0, 116.0), c.px(44.0), c.ink(ACCENT, 0.9));

        txt(c, 20.0, 152.0, "REGISTRA TU PRIMERA MOTO", 8.6, c.ink(MUTED, 1.0), SEMI);
        let _ = self.campos_vehiculo(c, ui, 170.0);

        if self.foco.is_some() && !cfg!(target_os = "android") {
            self.teclado(c, ui);
        } else {
            if boton(c, ui, c.rect(20.0, 700.0, 360.0, 48.0), "onb_ok", "Comenzar", true) {
                self.guardar_onboarding();
            }
            txt_c(
                c,
                200.0,
                766.0,
                "Tus datos se guardan solo en tu teléfono",
                8.4,
                c.ink(DIM, 1.0),
                REG,
            );
        }
    }

    fn guardar_onboarding(&mut self) {
        if self.form.marca.trim().is_empty() || self.form.modelo.trim().is_empty() {
            self.avisar("Escribe la marca y el modelo");
            return;
        }
        let year = solo_digitos(&self.form.year) as i32;
        let km = solo_digitos(&self.form.km);
        if !(1950..=datos::hoy().0).contains(&year) {
            self.avisar("Revisa el año del vehículo");
            return;
        }
        if km < 0 {
            self.avisar("El kilometraje no puede ser negativo");
            return;
        }
        self.datos.vehiculos.push(Vehiculo {
            marca: self.form.marca.trim().to_string(),
            modelo: self.form.modelo.trim().to_string(),
            year,
            km,
            placa: self.form.placa.trim().to_uppercase(),
            color: self.form.color.trim().to_string(),
            cc: solo_digitos(&self.form.cc) as i32,
            principal: true,
        });
        self.datos.sel = 0;
        self.guardar();
        self.tab = Tab::Inicio;
        self.ir(Pantalla::Principal);
        self.avisar("¡Listo! Tu garaje está activo");
    }

    // ================================================================ inicio
    fn inicio(&mut self, base: &Canvas, ui: &mut egui::Ui) {
        let e = ease_out(((self.clock - self.entrada) / 0.45) as f32);
        let c = &base.with(e, (1.0 - e) * 10.0);
        let ncols = self.lay.cols;
        let v_idx = self.datos.indice();
        let Some(v) = self.datos.veh().cloned() else {
            return;
        };

        let mut ir_manto = false;
        let mut ver_stats = false;
        let mut abrir_tanqueo = false;
        let mut abrir_servicio = false;
        let mut abrir_docs = false;

        // saludo
        txt(c, 20.0, 26.0, datos::saludo(), 11.5, c.ink(MUTED, 1.0), REG);
        txt(c, 20.0, 40.0, &self.datos.perfil.nombre_o(), 21.0, c.ink(TEXT, 1.0), BOLD);
        txt(c, 20.0, 68.0, &fecha_larga(), 9.6, c.ink(DIM, 1.0), MED);

        // botón asistente rápido / caja inteligente
        let r_ia = c.rect(276.0, 31.0, 60.0, 30.0);
        let resp_ia = ui.interact(r_ia, Id::new("top_ia"), Sense::click());
        c.p.rect_filled(r_ia, Rounding::same(c.px(15.0)), c.ink(SURF2, 1.0));
        c.p.rect_stroke(r_ia, Rounding::same(c.px(15.0)), Stroke::new(1.0, c.ink(ACCENT, 0.6)));
        icon(c, Ic::Bolt, c.pos(290.0, 46.0), c.px(12.0), c.ink(ACCENT, 1.0));
        txt(c, 304.0, 40.0, "IA", 10.0, c.ink(TEXT, 1.0), BOLD);
        if resp_ia.clicked() {
            self.abrir_caja();
        }

        // avatar
        let av = c.pos(364.0, 46.0);
        let resp_av = ui.interact(c.rect(345.0, 27.0, 38.0, 38.0), Id::new("top_avatar"), Sense::click());
        c.p.circle_filled(av, c.px(19.0), c.ink(SURF2, 1.0));
        c.p.circle_stroke(av, c.px(19.0), Stroke::new(1.0, c.ink(LINE, 1.0)));
        txt_sc(c, av, &self.datos.perfil.iniciales(), 11.0, c.ink(TEXT, 0.95), SEMI);
        c.p.circle_filled(
            Pos2::new(av.x + c.px(13.0), av.y - c.px(13.0)),
            c.px(3.0),
            c.ink(ACCENT, 1.0),
        );
        if resp_av.clicked() {
            self.tab = Tab::Perfil;
            self.entrada = self.clock;
            self.scroll = 0.0;
        }

        // vehículo
        txt(c, 20.0, 92.0, &v.titulo(), 20.0, c.ink(TEXT, 1.0), BOLD);
        match self.datos.estado() {
            Some(est) => {
                let col = if est >= 0.85 {
                    GREEN
                } else if est >= 0.6 {
                    AMBER
                } else {
                    ACCENT
                };
                chip(c, 296.0, 94.0, 84.0, 20.0, &format!("{} %", (est * 100.0) as i32), col, 0.16);
            }
            None => chip(c, 300.0, 94.0, 80.0, 20.0, "Nueva", ACCENT, 0.16),
        }
        let placa = if v.placa.trim().is_empty() {
            "sin placa".to_string()
        } else {
            v.placa.clone()
        };
        txt(
            c,
            20.0,
            118.0,
            &format!("{} · {} cc · {} · {}", v.year, v.cc, placa, km(v.km)),
            9.4,
            c.ink(MUTED, 1.0),
            MED,
        );

        // la moto flota sobre el fondo, sin recuadro
        let hueco = c.rect(0.0, 128.0, 400.0, 286.0);
        self.pintar_hero(c, hueco, 0.0);
        txt_c(c, 200.0, 392.0, "Toca la moto para ver su ficha", 8.4, c.ink(MUTED, 0.9), MED);
        let hero_hit = ui.interact(c.rect(0.0, 128.0, 400.0, 272.0), Id::new("hero"), Sense::click());
        if hero_hit.clicked() {
            self.ir(Pantalla::Detalle(v_idx));
        }

        // --- grupos B y C: en tablet van a la columna 1, uno tras otro.
        let c0 = c.clone();
        let c1 = if ncols >= 2 {
            c0.col(1).with(1.0, -380.0)
        } else {
            scrollable(&c0, ui, &mut self.scroll, 760.0, 406.0, self.lay.bottom)
        };
        let c = &c1;

        // tres datos del mes
        let stats = c.rect(20.0, 406.0, 360.0, 62.0);
        card(c, stats, 16.0, c.ink(SURF, 1.0), 1.0);
        let faltan = self.datos.proximo_servicio().map(|s| s.faltan(self.datos.km_actual()));
        let prox_txt = match faltan {
            Some(f) if f > 0 => km(f),
            Some(_) => "Vencido".to_string(),
            None => "—".to_string(),
        };
        let prox_col = if matches!(faltan, Some(f) if f <= 0) { ACCENT } else { TEXT };
        let consumo = self.datos.consumo();
        let cols = [
            ("Próximo servicio", prox_txt, prox_col),
            ("Gasto del mes", money(self.datos.gasto_mes()), TEXT),
            (
                "Consumo",
                if consumo > 0 {
                    format!("{consumo} km/gal")
                } else {
                    "—".to_string()
                },
                TEXT,
            ),
        ];
        for (i, (label, valor, col)) in cols.iter().enumerate() {
            let x = 20.0 + i as f32 * 120.0;
            if i > 0 {
                c.p.line_segment(
                    [c.pos(x, 419.0), c.pos(x, 455.0)],
                    Stroke::new(1.0, c.ink(LINE, 1.0)),
                );
            }
            txt_c(c, x + 60.0, 425.0, label, 8.2, c.ink(MUTED, 1.0), MED);
            txt_c(c, x + 60.0, 445.0, valor, 12.4, c.ink(*col, 1.0), SEMI);
        }

        // accesos rápidos
        txt(c, 20.0, 484.0, "ACCESOS RÁPIDOS", 8.6, c.ink(MUTED, 1.0), SEMI);
        let acciones = [
            (Ic::Fuel, "Tanquear", 0),
            (Ic::Wrench, "Servicio", 1),
            (Ic::Chart, "Gastos", 2),
            (Ic::Doc, "Documentos", 3),
        ];
        for (i, (ic, label, kind)) in acciones.iter().enumerate() {
            let cx = 66.0 + i as f32 * 89.0;
            let hit = c.rect(cx - 30.0, 496.0, 60.0, 60.0);
            let resp = ui.interact(hit, Id::new(("qa", i)), Sense::click());
            let press = if resp.is_pointer_button_down_on() { 0.96 } else { 1.0 };
            let circle = c.pos(cx, 518.0);
            c.p.circle_filled(circle, c.px(21.0 * press), c.ink(SURF2, 1.0));
            c.p.circle_stroke(circle, c.px(21.0 * press), Stroke::new(1.0, c.ink(LINE, 1.0)));
            icon(c, *ic, circle, c.px(17.0), c.ink(if i == 0 { ACCENT } else { TEXT }, 0.95));
            txt_c(c, cx, 551.0, label, 8.4, c.ink(MUTED, 1.0), MED);
            if resp.clicked() {
                match kind {
                    0 => abrir_tanqueo = true,
                    1 => abrir_servicio = true,
                    2 => ver_stats = true,
                    _ => abrir_docs = true,
                }
            }
        }

        // grupo C continúa en la misma columna, justo debajo del grupo B.
        let c = &c1;

        // mantenimiento
        let head = ui.interact(c.rect(280.0, 560.0, 100.0, 20.0), Id::new("ver_manto"), Sense::click());
        txt(c, 20.0, 564.0, "MANTENIMIENTO", 8.6, c.ink(MUTED, 1.0), SEMI);
        txt_r(c, 380.0, 564.0, "Ver todo", 9.4, c.ink(ACCENT, 0.95), SEMI);
        if head.clicked() {
            ir_manto = true;
        }

        let progreso = ease_out(((self.clock - self.entrada) / 0.7) as f32);
        let mut k = 0usize;
        let mut hay_servicios = false;
        for sv in self.datos.servicios.iter().filter(|s| s.vehiculo == v_idx) {
            if k >= 2 {
                break;
            }
            hay_servicios = true;
            let y = 580.0 + k as f32 * 50.0;
            let r = c.rect(20.0, y, 360.0, 44.0);
            card(c, r, 14.0, c.ink(SURF, 1.0), 1.0);
            let faltan = sv.faltan(self.datos.km_actual());
            let dotc = if faltan <= 0 { ACCENT } else { GREEN };
            c.p.circle_filled(c.pos(38.0, y + 15.0), c.px(3.0), c.ink(dotc, 1.0));
            txt(c, 50.0, y + 9.0, &sv.nombre, 11.2, c.ink(TEXT, 1.0), SEMI);
            txt_r(
                c,
                366.0,
                y + 9.0,
                &if faltan > 0 { km(faltan) } else { "Vencido".to_string() },
                9.8,
                c.ink(if faltan > 0 { MUTED } else { ACCENT }, 1.0),
                MED,
            );
            bar(c, 50.0, y + 28.0, 316.0, 4.0, sv.avance(self.datos.km_actual()) * progreso, dotc);
            k += 1;
        }
        if !hay_servicios {
            let r = c.rect(20.0, 580.0, 360.0, 44.0);
            card(c, r, 14.0, c.ink(SURF, 0.6), 1.0);
            txt(c, 36.0, 594.0, "Aún no has agregado servicios", 10.4, c.ink(MUTED, 1.0), MED);
        }

        // actividad reciente
        txt(c, 20.0, 686.0, "ACTIVIDAD RECIENTE", 8.6, c.ink(MUTED, 1.0), SEMI);
        let movs = self.datos.movimientos();
        if movs.is_empty() {
            let r = c.rect(20.0, 702.0, 360.0, 30.0);
            c.p.rect_filled(r, Rounding::same(c.px(10.0)), c.ink(SURF, 0.55));
            txt(c, 36.0, 711.0, "Sin movimientos todavía", 10.2, c.ink(MUTED, 1.0), MED);
        } else {
            for (i, m) in movs.iter().take(2).enumerate() {
                let y = 702.0 + i as f32 * 34.0;
                let r = c.rect(20.0, y, 360.0, 30.0);
                let resp_m = ui.interact(r, Id::new(("act_item", i)), Sense::click());
                c.p.rect_filled(r, Rounding::same(c.px(10.0)), c.ink(SURF, if resp_m.is_pointer_button_down_on() { 0.8 } else { 0.55 }));
                let ic = if m.tipo == TipoMov::Tanqueo { Ic::Fuel } else { Ic::Wrench };
                c.p.circle_filled(c.pos(36.0, y + 15.0), c.px(11.0), c.ink(SURF3, 0.9));
                icon(c, ic, c.pos(36.0, y + 15.0), c.px(11.0), c.ink(MUTED, 1.0));
                txt(c, 54.0, y + 9.0, &m.titulo, 10.2, c.ink(TEXT, 0.95), MED);
                txt_r(c, 316.0, y + 9.0, &money(m.valor), 10.2, c.ink(TEXT, 1.0), SEMI);
                txt_r(c, 368.0, y + 10.0, &m.fecha, 8.6, c.ink(DIM, 1.0), MED);
                if resp_m.clicked() {
                    if m.tipo == TipoMov::Tanqueo {
                        ver_stats = true;
                    } else {
                        ir_manto = true;
                    }
                }
            }
        }

        // aplicar acciones
        if abrir_tanqueo {
            self.abrir_form(FormKind::Tanqueo);
        } else if abrir_servicio {
            self.abrir_form(FormKind::ServicioNuevo);
        } else if abrir_docs {
            self.abrir_form(FormKind::Documento);
        } else if ir_manto {
            self.tab = Tab::Manto;
            self.entrada = self.clock;
            self.scroll = 0.0;
        } else if ver_stats {
            self.tab = Tab::Stats;
            self.entrada = self.clock;
            self.scroll = 0.0;
        }
    }

    // ============================================================== mi garaje
    fn garaje(&mut self, base: &Canvas, ui: &mut egui::Ui) {
        let e = ease_out(((self.clock - self.entrada) / 0.45) as f32);
        let n = self.datos.vehiculos.len();
        let v_idx = self.datos.indice();
        let mut abrir: Option<usize> = None;
        let mut editar_doc: Option<usize> = None;
        let mut nuevo = false;

        // cabecera fija
        let ch = base.with(e, (1.0 - e) * 10.0);
        txt(&ch, 20.0, 28.0, "Mi garaje", 22.0, ch.ink(TEXT, 1.0), BOLD);
        let ciudad = if self.datos.perfil.ciudad.trim().is_empty() {
            "Bogotá D.C."
        } else {
            self.datos.perfil.ciudad.trim()
        };
        txt(
            &ch,
            20.0,
            58.0,
            &format!("{} {} · {}", n, if n == 1 { "vehículo" } else { "vehículos" }, ciudad),
            10.0,
            ch.ink(MUTED, 1.0),
            MED,
        );

        let docs_n = self.datos.docs_de(v_idx).len();
        let mut alto = 84.0 + 16.0 + n as f32 * 150.0 + 70.0;
        alto += 16.0 + 96.0 + 22.0;
        alto += 16.0 + 88.0 + 22.0;
        alto += 62.0 + 22.0;
        alto += 16.0 + (docs_n.max(1) as f32 * 44.0) + 24.0;
        let c = &scrollable(&ch, ui, &mut self.scroll, alto, self.lay.top, self.lay.bottom);

        let mut y = 84.0;
        txt(c, 20.0, y, "TUS VEHÍCULOS", 8.6, c.ink(MUTED, 1.0), SEMI);
        y += 16.0;

        for i in 0..n {
            let r = c.rect(20.0, y, 360.0, 140.0);
            card(c, r, 18.0, c.ink(SURF, 1.0), 1.0);
            let resp = ui.interact(r, Id::new(("veh", i)), Sense::click());

            let ph = c.rect(30.0, y + 10.0, 108.0, 120.0);
            if self.datos.vehiculos[i].principal {
                self.pintar_foto(c, ph, 12.0);
            } else {
                c.p.rect_filled(ph, Rounding::same(c.px(12.0)), c.ink(SURF2, 1.0));
                silueta(c, ph);
            }

            let v = &self.datos.vehiculos[i];
            let x = 150.0;
            txt(c, x, y + 16.0, &v.marca.to_uppercase(), 8.2, c.ink(MUTED, 1.0), SEMI);
            txt(c, x, y + 28.0, &v.modelo, 16.0, c.ink(TEXT, 1.0), BOLD);
            txt(
                c,
                x,
                y + 52.0,
                &format!("{} · {}", v.year, if v.placa.trim().is_empty() { "sin placa" } else { v.placa.trim() }),
                9.6,
                c.ink(MUTED, 1.0),
                MED,
            );
            txt(c, x, y + 78.0, &km(v.km), 11.0, c.ink(TEXT, 1.0), SEMI);
            if v.principal {
                chip(c, 296.0, y + 106.0, 54.0, 18.0, "Activa", GREEN, 0.16);
            }
            icon(c, Ic::Chevron, c.pos(340.0, y + 68.0), c.px(14.0), c.ink(DIM, 1.0));

            if resp.clicked() {
                abrir = Some(i);
            }
            y += 150.0;
        }

        // agregar vehículo
        if boton(c, ui, c.rect(20.0, y, 360.0, 54.0), "agregar", "＋  Agregar vehículo", false) {
            nuevo = true;
        }
        y += 70.0;

        // resumen
        txt(c, 20.0, y, "RESUMEN DEL GARAJE", 8.6, c.ink(MUTED, 1.0), SEMI);
        let r2 = c.rect(20.0, y + 16.0, 360.0, 96.0);
        card(c, r2, 16.0, c.ink(SURF, 1.0), 1.0);
        let filas = [
            ("Vehículos activos", format!("{n}")),
            ("Kilometraje total", km(self.datos.vehiculos.iter().map(|v| v.km).sum())),
            ("Gasto del mes", money(self.datos.gasto_mes())),
        ];
        for (i, (k, v)) in filas.iter().enumerate() {
            let yy = y + 16.0 + i as f32 * 30.0;
            txt(c, 36.0, yy + 8.0, k, 10.0, c.ink(MUTED, 1.0), REG);
            txt_r(c, 364.0, yy + 6.0, v, 11.0, c.ink(TEXT, 1.0), SEMI);
            if i < 2 {
                divider(c, 36.0, 364.0, yy + 27.0, 0.6);
            }
        }
        y += 16.0 + 96.0 + 22.0;

        // próximo en el garaje
        txt(c, 20.0, y, "PRÓXIMO EN EL GARAJE", 8.6, c.ink(MUTED, 1.0), SEMI);
        let r3 = c.rect(20.0, y + 16.0, 360.0, 88.0);
        card(c, r3, 16.0, c.ink(SURF, 1.0), 1.0);
        if let Some(sv) = self.datos.proximo_servicio() {
            icon(c, Ic::Wrench, c.pos(44.0, y + 44.0), c.px(17.0), c.ink(ACCENT, 0.9));
            txt(c, 66.0, y + 30.0, &sv.nombre, 11.5, c.ink(TEXT, 1.0), SEMI);
            txt(c, 66.0, y + 48.0, &sv.detalle, 8.8, c.ink(MUTED, 1.0), REG);
            txt_r(c, 364.0, y + 32.0, &money(sv.costo), 11.0, c.ink(TEXT, 1.0), SEMI);
            let faltan = sv.faltan(self.datos.km_actual());
            txt(
                c,
                66.0,
                y + 72.0,
                &if faltan > 0 { format!("Faltan {}", km(faltan)) } else { "Vencido".to_string() },
                8.8,
                c.ink(if faltan > 0 { AMBER } else { ACCENT }, 1.0),
                SEMI,
            );
            bar(c, 190.0, y + 76.0, 174.0, 4.0, sv.avance(self.datos.km_actual()) * e, ACCENT);
        } else {
            txt(c, 66.0, y + 44.0, "Sin servicios programados", 10.4, c.ink(MUTED, 1.0), MED);
        }
        y += 16.0 + 88.0 + 22.0;

        // consejo
        let r4 = c.rect(20.0, y, 360.0, 62.0);
        card(c, r4, 16.0, c.ink(SURF, 1.0), 1.0);
        icon(c, Ic::Fuel, c.pos(44.0, y + 31.0), c.px(18.0), c.ink(ACCENT, 0.9));
        txt(c, 66.0, y + 15.0, "Registra cada tanqueo", 10.8, c.ink(TEXT, 1.0), SEMI);
        txt(
            c,
            66.0,
            y + 33.0,
            "Con cada carga calculamos tu consumo real.",
            8.8,
            c.ink(MUTED, 1.0),
            REG,
        );
        y += 62.0 + 22.0;

        // documentación
        txt(c, 20.0, y, "DOCUMENTACIÓN", 8.6, c.ink(MUTED, 1.0), SEMI);
        let docs: Vec<Documento> = self.datos.docs.iter().filter(|d| d.vehiculo == v_idx).cloned().collect();
        if docs.is_empty() {
            let r5 = c.rect(20.0, y + 16.0, 360.0, 44.0);
            card(c, r5, 16.0, c.ink(SURF, 0.6), 1.0);
            txt(c, 36.0, y + 30.0, "Sin documentos registrados", 10.4, c.ink(MUTED, 1.0), MED);
        } else {
            let r5 = c.rect(20.0, y + 16.0, 360.0, docs.len() as f32 * 44.0);
            card(c, r5, 16.0, c.ink(SURF, 1.0), 1.0);
            for (i, d) in docs.iter().enumerate() {
                let yy = y + 16.0 + i as f32 * 44.0;
                let col = if d.vigente() { GREEN } else { AMBER };
                c.p.circle_filled(c.pos(40.0, yy + 22.0), c.px(11.0), c.ink(col, 0.16));
                icon(c, Ic::Check, c.pos(40.0, yy + 22.0), c.px(11.0), c.ink(col, 1.0));
                txt(c, 60.0, yy + 9.0, &d.nombre, 11.0, c.ink(TEXT, 1.0), SEMI);
                txt(c, 60.0, yy + 25.0, &format!("Vence {}", d.vence), 8.8, c.ink(MUTED, 1.0), REG);
                txt_r(
                    c,
                    364.0,
                    yy + 14.0,
                    if d.vigente() { "Vigente" } else { "Vencido" },
                    9.2,
                    c.ink(col, 1.0),
                    SEMI,
                );
                if i + 1 < docs.len() {
                    divider(c, 60.0, 364.0, yy + 43.0, 0.6);
                }
                let hit_doc = c.rect(20.0, yy, 360.0, 44.0);
                let resp_d = ui.interact(hit_doc, Id::new(("gar_doc", i)), Sense::click());
                if resp_d.clicked() {
                    editar_doc = Some(i);
                }
            }
        }

        if let Some(i) = abrir {
            self.ir(Pantalla::Detalle(i));
        } else if let Some(i) = editar_doc {
            self.abrir_form(FormKind::DocumentoEditar(i));
        } else if nuevo {
            self.abrir_form(FormKind::VehiculoNuevo);
        }
    }

    // ========================================================= mantenimiento
    fn mantenimiento(&mut self, base: &Canvas, ui: &mut egui::Ui) {
        let e = ease_out(((self.clock - self.entrada) / 0.45) as f32);
        let v_idx = self.datos.indice();
        let mut marcar: Option<usize> = None;
        let mut editar: Option<usize> = None;
        let mut editar_doc: Option<usize> = None;
        let mut nuevo = false;
        let mut nuevo_doc = false;
        let mut toggle_rec = false;

        let servs: Vec<(usize, Servicio)> = self
            .datos
            .servicios
            .iter()
            .enumerate()
            .filter(|(_, s)| s.vehiculo == v_idx)
            .map(|(i, s)| (i, s.clone()))
            .collect();
        let docs: Vec<Documento> = self.datos.docs.iter().filter(|d| d.vehiculo == v_idx).cloned().collect();

        let ch = base.with(e, (1.0 - e) * 10.0);
        txt(&ch, 20.0, 28.0, "Mantenimiento", 22.0, ch.ink(TEXT, 1.0), BOLD);
        txt(&ch, 20.0, 58.0, "Próximos servicios y documentos", 10.0, ch.ink(MUTED, 1.0), MED);

        let mut alto = 176.0 + servs.len() as f32 * 84.0 + 62.0;
        alto += 16.0 + docs.len().max(1) as f32 * 52.0 + 10.0;
        alto += 60.0 + 10.0 + 48.0 + 10.0 + 48.0 + 30.0;
        let c = &scrollable(&ch, ui, &mut self.scroll, alto, self.lay.top, self.lay.bottom);

        // resumen
        let km_actual = self.datos.km_actual();
        let vencidos = servs.iter().filter(|(_, s)| s.faltan(km_actual) <= 0).count();
        let r = c.rect(20.0, 84.0, 360.0, 62.0);
        card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
        c.p.circle_filled(c.pos(44.0, 115.0), c.px(16.0), c.ink(ACCENT, 0.16));
        icon(c, Ic::Gauge, c.pos(44.0, 115.0), c.px(16.0), c.ink(ACCENT, 1.0));
        txt(c, 70.0, 100.0, "Estado del plan", 9.4, c.ink(MUTED, 1.0), MED);
        let frase = if servs.is_empty() {
            "Sin servicios programados".to_string()
        } else if vencidos == 0 {
            format!("{} servicios · al día", servs.len())
        } else {
            format!("{vencidos} vencidos de {}", servs.len())
        };
        txt(c, 70.0, 112.0, &frase, 12.0, c.ink(TEXT, 1.0), SEMI);
        if !servs.is_empty() {
            let pct = ((servs.len() - vencidos) as f32 / servs.len() as f32 * 100.0) as i32;
            let col = if vencidos == 0 { GREEN } else { AMBER };
            txt_r(c, 364.0, 106.0, &format!("{pct} %"), 16.0, c.ink(col, 1.0), NUM);
        }

        txt(c, 20.0, 158.0, "PRÓXIMOS SERVICIOS", 8.6, c.ink(MUTED, 1.0), SEMI);

        let progreso = ease_out(((self.clock - self.entrada) / 0.7) as f32);
        for (k, (idx, sv)) in servs.iter().enumerate() {
            let y = 176.0 + k as f32 * 84.0;
            let r = c.rect(20.0, y, 360.0, 76.0);
            card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
            let faltan = sv.faltan(km_actual);
            let col = if faltan <= 0 { ACCENT } else { GREEN };
            txt(c, 36.0, y + 12.0, &sv.nombre, 12.5, c.ink(TEXT, 1.0), SEMI);
            txt_r(c, 364.0, y + 13.0, &money(sv.costo), 10.4, c.ink(MUTED, 1.0), SEMI);
            txt(
                c,
                36.0,
                y + 30.0,
                &if sv.detalle.trim().is_empty() {
                    format!("Cada {}", km(sv.cada_km))
                } else {
                    sv.detalle.clone()
                },
                9.2,
                c.ink(MUTED, 1.0),
                REG,
            );
            txt(
                c,
                36.0,
                y + 52.0,
                &if faltan > 0 { format!("Faltan {}", km(faltan)) } else { "Vencido".to_string() },
                9.4,
                c.ink(if faltan > 0 { MUTED } else { ACCENT }, 1.0),
                SEMI,
            );
            bar(c, 130.0, y + 55.0, 122.0, 4.0, sv.avance(km_actual) * progreso, col);

            if boton(c, ui, c.rect(288.0, y + 40.0, 76.0, 26.0), &format!("mark{idx}"), "HECHO", false) {
                marcar = Some(*idx);
            }
            let ed = ui.interact(c.rect(230.0, y + 40.0, 50.0, 26.0), Id::new(("edit", idx)), Sense::click());
            icon(c, Ic::Pencil, c.pos(255.0, y + 53.0), c.px(13.0), c.ink(MUTED, 1.0));
            if ed.clicked() {
                editar = Some(*idx);
            }
        }
        if servs.is_empty() {
            let r = c.rect(20.0, 176.0, 360.0, 62.0);
            card(c, r, 16.0, c.ink(SURF, 0.6), 1.0);
            txt(c, 36.0, 200.0, "Aún no has agregado servicios", 10.6, c.ink(MUTED, 1.0), MED);
        }
        let mut y = 176.0 + servs.len().max(1) as f32 * 84.0 + 6.0;

        // documentación
        icon(c, Ic::Calendar, c.pos(25.0, y + 4.0), c.px(11.0), c.ink(MUTED, 1.0));
        txt(c, 36.0, y, "DOCUMENTACIÓN", 8.6, c.ink(MUTED, 1.0), SEMI);
        if docs.is_empty() {
            let r = c.rect(20.0, y + 16.0, 360.0, 44.0);
            card(c, r, 14.0, c.ink(SURF, 0.6), 1.0);
            txt(c, 36.0, y + 30.0, "Sin documentos registrados", 10.4, c.ink(MUTED, 1.0), MED);
        } else {
            for (k, d) in docs.iter().enumerate() {
                let yy = y + 16.0 + k as f32 * 52.0;
                let r = c.rect(20.0, yy, 360.0, 44.0);
                card(c, r, 14.0, c.ink(SURF, 1.0), 1.0);
                let col = if d.vigente() { GREEN } else { AMBER };
                c.p.circle_filled(c.pos(40.0, yy + 22.0), c.px(11.0), c.ink(col, 0.16));
                icon(c, Ic::Check, c.pos(40.0, yy + 22.0), c.px(11.0), c.ink(col, 1.0));
                txt(c, 60.0, yy + 9.0, &d.nombre, 11.5, c.ink(TEXT, 1.0), SEMI);
                txt(c, 60.0, yy + 25.0, &format!("Vence {}", d.vence), 9.0, c.ink(MUTED, 1.0), REG);
                txt_r(
                    c,
                    342.0,
                    yy + 14.0,
                    if d.vigente() { "Vigente" } else { "Vencido" },
                    9.4,
                    c.ink(col, 1.0),
                    SEMI,
                );
                icon(c, Ic::Chevron, c.pos(364.0, yy + 22.0), c.px(11.0), c.ink(MUTED, 1.0));
                let resp_d = ui.interact(r, Id::new(("doc_manto", k)), Sense::click());
                if resp_d.clicked() {
                    editar_doc = Some(k);
                }
            }
        }
        y += 16.0 + docs.len().max(1) as f32 * 52.0 + 10.0;

        // recordatorios
        let r = c.rect(20.0, y, 360.0, 60.0);
        card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
        icon(c, Ic::Bell, c.pos(44.0, y + 30.0), c.px(17.0), c.ink(MUTED, 1.0));
        txt(c, 66.0, y + 14.0, "Recordatorios", 11.5, c.ink(TEXT, 1.0), SEMI);
        txt(c, 66.0, y + 33.0, "Avisos por kilometraje y fechas", 8.8, c.ink(MUTED, 1.0), REG);
        let on = self.datos.recordatorios;
        let sw = c.rect(310.0, y + 20.0, 44.0, 22.0);
        let resp = ui.interact(sw, Id::new("sw_rec"), Sense::click());
        c.p.rect_filled(sw, Rounding::same(c.px(11.0)), c.ink(if on { ACCENT } else { SURF3 }, 1.0));
        let knob = c.pos(if on { 343.0 } else { 321.0 }, y + 31.0);
        c.p.circle_filled(knob, c.px(9.0), c.ink(Color32::WHITE, 0.95));
        if resp.clicked() {
            toggle_rec = true;
        }
        y += 70.0;

        // acciones
        if boton(c, ui, c.rect(20.0, y, 174.0, 48.0), "add_serv", "＋  Servicio", true) {
            nuevo = true;
        }
        if boton(c, ui, c.rect(206.0, y, 174.0, 48.0), "add_doc", "＋  Documento", false) {
            nuevo_doc = true;
        }

        if let Some(i) = marcar {
            self.marcar_servicio(i);
        } else if let Some(i) = editar {
            self.abrir_form(FormKind::ServicioEditar(i));
        } else if let Some(i) = editar_doc {
            self.abrir_form(FormKind::DocumentoEditar(i));
        } else if nuevo {
            self.abrir_form(FormKind::ServicioNuevo);
        } else if nuevo_doc {
            self.abrir_form(FormKind::Documento);
        } else if toggle_rec {
            self.datos.recordatorios = !self.datos.recordatorios;
            self.guardar();
            self.avisar(if self.datos.recordatorios {
                "Recordatorios activados"
            } else {
                "Recordatorios desactivados"
            });
        }
    }

    // =========================================================== estadísticas
    fn estadisticas(&mut self, base: &Canvas, ui: &mut egui::Ui) {
        let e = ease_out(((self.clock - self.entrada) / 0.45) as f32);
        let ch = base.with(e, (1.0 - e) * 10.0);
        let ncols = self.lay.cols;
        txt(&ch, 20.0, 28.0, "Estadísticas", 22.0, ch.ink(TEXT, 1.0), BOLD);
        txt(&ch, 20.0, 58.0, "Evolución de tu moto · últimos 7 meses", 10.0, ch.ink(MUTED, 1.0), MED);

        // En tablet no hay scroll: cada grupo de tarjetas vive en su columna.
        let sc = if ncols == 1 {
            scrollable(&ch, ui, &mut self.scroll, 700.0, self.lay.top, self.lay.bottom)
        } else {
            ch.clone()
        };
        let c = &sc;

        // total del mes
        let r = c.rect(20.0, 84.0, 360.0, 104.0);
        card(c, r, 18.0, c.ink(SURF, 1.0), 1.0);
        grad_v(
            c,
            c.rect(20.0, 84.0, 360.0, 104.0),
            a(c.ink(ACCENT, 1.0), 0.10),
            a(c.ink(SURF, 1.0), 0.0),
        );
        let mes = self.datos.gasto_mes();
        let anterior = self.datos.gasto_mes_anterior();
        txt(c, 36.0, 98.0, "GASTO TOTAL DEL MES", 8.6, c.ink(MUTED, 1.0), SEMI);
        txt(c, 36.0, 112.0, &money(mes), 27.0, c.ink(TEXT, 1.0), NUM);
        if anterior > 0.0 {
            let d = (mes - anterior) / anterior * 100.0;
            let (txto, col) = if d >= 0.0 {
                (format!("↑ {:.1} % frente al mes pasado", d), ACCENT)
            } else {
                (format!("↓ {:.1} % frente al mes pasado", -d), GREEN)
            };
            txt(c, 36.0, 150.0, &txto, 9.2, c.ink(col, 1.0), MED);
        } else {
            txt(c, 36.0, 150.0, "Sin historial del mes anterior", 9.2, c.ink(MUTED, 1.0), MED);
        }
        chip(c, 268.0, 98.0, 96.0, 22.0, "Este mes", TEXT, 0.10);

        // gráfico de barras
        let chart = c.rect(20.0, 200.0, 360.0, 186.0);
        card(c, chart, 18.0, c.ink(SURF, 1.0), 1.0);
        txt(c, 36.0, 214.0, "GASTO MENSUAL", 8.6, c.ink(MUTED, 1.0), SEMI);
        txt_r(c, 364.0, 214.0, "COP", 8.6, c.ink(DIM, 1.0), SEMI);

        let serie = self.datos.serie_meses();
        let max = serie.iter().map(|(_, v)| *v).fold(0.0f64, f64::max).max(1.0);
        let base_y = 352.0;
        for g in 0..4 {
            let y = 244.0 + g as f32 * 36.0;
            c.p.line_segment(
                [c.pos(36.0, y), c.pos(364.0, y)],
                Stroke::new(1.0, c.ink(LINE, 0.55)),
            );
        }
        for (i, (label, v)) in serie.iter().enumerate() {
            let x = 44.0 + i as f32 * 46.0;
            let anim = ease_out(((self.clock - self.entrada) / 0.7 - i as f64 * 0.05) as f32);
            let h = ((v / max) as f32) * 96.0 * anim;
            let rr = c.rect(x, base_y - h, 26.0, h);
            let ultimo = i + 1 == serie.len();
            let col = if ultimo { ACCENT } else { SURF3 };
            c.p.rect_filled(rr, Rounding::same(c.px(6.0)), c.ink(col, 1.0));
            if ultimo {
                c.p.rect_filled(
                    Rect::from_min_size(rr.min, Vec2::new(rr.width(), rr.height() * 0.4)),
                    Rounding::same(c.px(6.0)),
                    c.ink(Color32::WHITE, 0.12),
                );
            }
            txt_c(c, x + 13.0, 364.0, label, 8.2, c.ink(if ultimo { TEXT } else { DIM }, 1.0), SEMI);
        }

        // --- grupos B y C: columna 1 en tablet, uno tras otro.
        let c0 = c.clone();
        let c1 = if ncols >= 2 {
            c0.col(1).with(1.0, -318.0)
        } else {
            c0.clone()
        };
        let c = &c1;

        // dos datos: rendimiento en km/galón y en pesos
        let consumo = self.datos.consumo();
        let km10 = self.datos.km_por_10mil();
        let tiles = [
            ("RINDE", if consumo > 0 { format!("{consumo} km/gal") } else { "—".to_string() }, Ic::Bolt),
            ("POR $10.000", if km10 > 0 { km(km10) } else { "—".to_string() }, Ic::Gauge),
        ];
        for (i, (label, valor, ic)) in tiles.iter().enumerate() {
            let x = 20.0 + i as f32 * 186.0;
            let r = c.rect(x, 402.0, 174.0, 76.0);
            card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
            icon(c, *ic, c.pos(x + 28.0, 424.0), c.px(16.0), c.ink(ACCENT, 0.9));
            txt(c, x + 16.0, 440.0, label, 8.4, c.ink(MUTED, 1.0), SEMI);
            txt(c, x + 16.0, 454.0, valor, 13.5, c.ink(TEXT, 1.0), SEMI);
        }

        // grupo C continúa en la misma columna, justo debajo del grupo B.
        let c = &c1;

        // evolución de kilometraje
        let km_card = c.rect(20.0, 494.0, 360.0, 168.0);
        card(c, km_card, 18.0, c.ink(SURF, 1.0), 1.0);
        txt(c, 36.0, 508.0, "KILOMETRAJE ACUMULADO", 8.6, c.ink(MUTED, 1.0), SEMI);
        txt_r(c, 364.0, 506.0, &km(self.datos.km_actual()), 12.0, c.ink(TEXT, 1.0), NUM);

        let kmserie = self.datos.serie_km();
        let vals: Vec<i64> = kmserie.iter().map(|(_, v)| *v).filter(|v| *v > 0).collect();
        if vals.len() >= 2 {
            let kmin = *vals.iter().min().unwrap();
            let kmax = *vals.iter().max().unwrap();
            let rango = (kmax - kmin).max(1) as f32;
            let mut pts: Vec<Pos2> = Vec::new();
            for (i, (_, v)) in kmserie.iter().enumerate() {
                let x = 44.0 + i as f32 * 52.0;
                let yy = 630.0 - ((*v - kmin) as f32 / rango) * 82.0;
                pts.push(c.pos(x, yy));
            }
            let anim = ease_out(((self.clock - self.entrada) / 0.8) as f32);
            let n = ((pts.len() as f32 - 1.0) * anim).ceil().max(1.0) as usize;
            let shown: Vec<Pos2> = pts.iter().take(n.min(pts.len())).cloned().collect();
            if shown.len() >= 2 {
                let mut poly = shown.clone();
                poly.push(Pos2::new(shown[shown.len() - 1].x, c.pos(0.0, 632.0).y));
                poly.push(Pos2::new(shown[0].x, c.pos(0.0, 632.0).y));
                c.p.add(egui::Shape::convex_polygon(poly, c.ink(ACCENT, 0.12), Stroke::NONE));
                c.p.add(egui::Shape::line(shown.clone(), Stroke::new(2.0, c.ink(ACCENT, 1.0))));
                if let Some(last) = shown.last() {
                    c.p.circle_filled(*last, c.px(3.6), c.ink(ACCENT, 1.0));
                    c.p.circle_filled(*last, c.px(6.4), c.ink(ACCENT, 0.22));
                }
            }
        } else {
            txt(c, 36.0, 560.0, "Registra al menos 2 tanqueos", 10.0, c.ink(MUTED, 1.0), MED);
            txt(c, 36.0, 578.0, "para ver tu kilometraje", 10.0, c.ink(MUTED, 1.0), MED);
        }
        for (i, (label, _)) in kmserie.iter().enumerate() {
            let x = 44.0 + i as f32 * 52.0;
            txt_c(c, x, 646.0, label, 7.6, c.ink(DIM, 1.0), SEMI);
        }

        c.p.circle_filled(c.pos(26.0, 684.0), c.px(3.0), c.ink(GREEN, 1.0));
        txt(c, 36.0, 678.0, "Datos guardados en tu equipo · sin cuentas", 8.8, c.ink(DIM, 1.0), REG);
    }

    // =============================================================== perfil
    fn perfil(&mut self, base: &Canvas, ui: &mut egui::Ui) {
        let e = ease_out(((self.clock - self.entrada) / 0.45) as f32);
        let ch = base.with(e, (1.0 - e) * 10.0);
        txt(&ch, 20.0, 28.0, "Perfil", 22.0, ch.ink(TEXT, 1.0), BOLD);
        let c = &scrollable(&ch, ui, &mut self.scroll, 740.0, self.lay.top, self.lay.bottom);

        let mut toggle_notif = false;
        let mut toggle_rec = false;
        let mut editar_perfil = false;
        let mut editar_precio = false;
        let mut borrar = false;

        // tarjeta de usuario
        let r = c.rect(20.0, 64.0, 360.0, 96.0);
        card(c, r, 18.0, c.ink(SURF, 1.0), 1.0);
        let av = c.pos(58.0, 112.0);
        c.p.circle_filled(av, c.px(26.0), c.ink(SURF3, 1.0));
        c.p.circle_stroke(av, c.px(26.0), Stroke::new(1.4, c.ink(ACCENT, 0.9)));
        txt_sc(c, av, &self.datos.perfil.iniciales(), 14.0, c.ink(TEXT, 1.0), SEMI);
        txt(c, 96.0, 92.0, &self.datos.perfil.nombre_o(), 15.5, c.ink(TEXT, 1.0), BOLD);
        let correo = if self.datos.perfil.email.trim().is_empty() {
            "sin correo"
        } else {
            self.datos.perfil.email.trim()
        };
        txt(c, 96.0, 114.0, correo, 9.8, c.ink(MUTED, 1.0), REG);
        chip(c, 96.0, 130.0, 92.0, 18.0, "PROPIETARIO", ACCENT, 0.16);

        // ajustes
        txt(c, 20.0, 180.0, "AJUSTES", 8.6, c.ink(MUTED, 1.0), SEMI);
        let filas_y = 196.0;
        let items = [
            ("Notificaciones", String::new()),
            ("Recordatorios", String::new()),
            (
                "Precio del galón",
                if self.datos.precio_galon > 0.0 {
                    money(self.datos.precio_galon)
                } else {
                    "Sin definir".to_string()
                },
            ),
            ("Unidades", "km · galones".to_string()),
            ("Moneda", "COP $".to_string()),
        ];
        let card_r = c.rect(20.0, filas_y, 360.0, 44.0 * items.len() as f32);
        card(c, card_r, 16.0, c.ink(SURF, 1.0), 1.0);
        for (i, (k, v)) in items.iter().enumerate() {
            let y = filas_y + i as f32 * 44.0;
            txt(c, 36.0, y + 15.0, k, 11.2, c.ink(TEXT, 0.95), MED);
            if i < 2 {
                let on = if i == 0 { self.datos.notif } else { self.datos.recordatorios };
                let sw = c.rect(318.0, y + 11.0, 44.0, 22.0);
                let resp = ui.interact(sw, Id::new(("per_sw", i)), Sense::click());
                c.p.rect_filled(sw, Rounding::same(c.px(11.0)), c.ink(if on { ACCENT } else { SURF3 }, 1.0));
                let knob = c.pos(if on { 351.0 } else { 329.0 }, y + 22.0);
                c.p.circle_filled(knob, c.px(9.0), c.ink(Color32::WHITE, 0.95));
                if resp.clicked() {
                    if i == 0 {
                        toggle_notif = true;
                    } else {
                        toggle_rec = true;
                    }
                }
            } else if i == 2 {
                let hit = c.rect(36.0, y, 324.0, 44.0);
                let resp = ui.interact(hit, Id::new("per_precio"), Sense::click());
                txt_r(c, 356.0, y + 16.0, v, 10.0, c.ink(ACCENT, 0.95), MED);
                icon(c, Ic::Chevron, c.pos(366.0, y + 22.0), c.px(11.0), c.ink(MUTED, 1.0));
                if resp.clicked() {
                    editar_precio = true;
                }
            } else {
                txt_r(c, 364.0, y + 16.0, v, 10.0, c.ink(MUTED, 1.0), MED);
            }
            if i + 1 < items.len() {
                divider(c, 36.0, 364.0, y + 43.0, 0.6);
            }
        }

        // datos de la cuenta
        let dy = filas_y + 44.0 * items.len() as f32 + 22.0;
        txt(c, 20.0, dy, "DATOS DE LA CUENTA", 8.6, c.ink(MUTED, 1.0), SEMI);
        let r2 = c.rect(20.0, dy + 16.0, 360.0, 96.0);
        card(c, r2, 16.0, c.ink(SURF, 1.0), 1.0);
        let ciudad = if self.datos.perfil.ciudad.trim().is_empty() {
            "—".to_string()
        } else {
            self.datos.perfil.ciudad.clone()
        };
        let tel = if self.datos.perfil.telefono.trim().is_empty() {
            "—".to_string()
        } else {
            self.datos.perfil.telefono.clone()
        };
        let datos_cuenta = [
            ("Ciudad", ciudad),
            ("Teléfono", tel),
            ("Correo", self.datos.perfil.email.clone()),
        ];
        for (i, (k, v)) in datos_cuenta.iter().enumerate() {
            let y = dy + 16.0 + i as f32 * 30.0;
            txt(c, 36.0, y + 8.0, k, 9.8, c.ink(MUTED, 1.0), REG);
            txt_r(c, 364.0, y + 7.0, v, 10.2, c.ink(TEXT, 0.95), MED);
            if i < 2 {
                divider(c, 36.0, 364.0, y + 27.0, 0.6);
            }
        }

        // botones
        let by = dy + 16.0 + 96.0 + 18.0;
        let mut exportar = false;
        if boton(c, ui, c.rect(20.0, by, 174.0, 48.0), "ed_perfil", "Editar perfil", true) {
            editar_perfil = true;
        }
        let etiqueta_borrar = if self.confirmando("borrar") {
            "¿Seguro? Toca otra vez"
        } else {
            "Borrar mis datos"
        };
        let rb = ui.interact(c.rect(206.0, by, 174.0, 48.0), Id::new("borrar"), Sense::click());
        c.p.rect_filled(
            c.rect(206.0, by, 174.0, 48.0),
            Rounding::same(c.px(14.0)),
            c.ink(ACCENT, 0.10),
        );
        c.p.rect_stroke(
            c.rect(206.0, by, 174.0, 48.0),
            Rounding::same(c.px(14.0)),
            Stroke::new(1.0, c.ink(ACCENT, 0.4)),
        );
        txt_c(c, 293.0, by + 24.0, etiqueta_borrar, 9.6, c.ink(ACCENT, 0.95), SEMI);
        if rb.clicked() {
            borrar = true;
        }

        let by2 = by + 56.0;
        if boton(c, ui, c.rect(20.0, by2, 360.0, 44.0), "exp_backup", "💾  Exportar respaldo (JSON)", false) {
            exportar = true;
        }

        // acerca de
        let ay = by2 + 44.0 + 18.0;
        let r3 = c.rect(20.0, ay, 360.0, 56.0);
        card(c, r3, 16.0, c.ink(SURF, 1.0), 1.0);
        icon(c, Ic::Bike, c.pos(44.0, ay + 28.0), c.px(18.0), c.ink(ACCENT, 0.9));
        txt(c, 66.0, ay + 13.0, "Mi Garaje", 11.5, c.ink(TEXT, 1.0), SEMI);
        txt(c, 66.0, ay + 32.0, "Versión 1.0 · garaje digital", 8.8, c.ink(MUTED, 1.0), REG);
        txt_c(c, 200.0, ay + 76.0, "Mi Garaje · garaje digital para tu moto", 8.6, c.ink(DIM, 1.0), REG);

        if toggle_notif {
            self.datos.notif = !self.datos.notif;
            self.guardar();
            self.avisar(if self.datos.notif {
                "Notificaciones activadas"
            } else {
                "Notificaciones desactivadas"
            });
        } else if toggle_rec {
            self.datos.recordatorios = !self.datos.recordatorios;
            self.guardar();
            self.avisar(if self.datos.recordatorios {
                "Recordatorios activados"
            } else {
                "Recordatorios desactivados"
            });
        } else if exportar {
            match datos::a_json(&self.datos) {
                Ok(j) => {
                    let _ = std::fs::write("MiGaraje_respaldo.json", j);
                    self.avisar("Respaldo guardado con éxito");
                }
                Err(e) => self.avisar(&e),
            }
        } else if editar_perfil {
            self.abrir_form(FormKind::Perfil);
        } else if editar_precio {
            self.abrir_form(FormKind::Precio);
        } else if borrar && self.confirmar("borrar") {
            self.borrar_todo();
        }
    }

    // =============================================================== detalle
    fn detalle(&mut self, base: &Canvas, ui: &mut egui::Ui, idx: usize) {
        let e = ease_out(((self.clock - self.entrada) / 0.4) as f32);
        let n = self.datos.vehiculos.len();
        let i = idx.min(n.saturating_sub(1));
        let Some(v) = self.datos.vehiculos.get(i).cloned() else {
            self.ir(Pantalla::Principal);
            return;
        };
        let c = &base.with(e, (1.0 - e) * 14.0);
        let km_actual = v.km;

        let mut volver = false;
        let mut editar = false;
        let mut eliminar = false;
        let mut ir_stats = false;

        // ---- la moto, sin fondo, flotando
        let hueco = c.rect(0.0, 76.0, 400.0, 262.0);
        self.pintar_hero(c, hueco, 0.0);
        grad_v(
            c,
            c.rect(0.0, 268.0, 400.0, 70.0),
            a(c.ink(BG, 1.0), 0.0),
            a(c.ink(BG, 1.0), 0.95),
        );
        if v.principal {
            chip(c, 20.0, 86.0, 82.0, 20.0, "PRINCIPAL", TEXT, 0.10);
        }
        txt(c, 20.0, 292.0, &v.titulo(), 16.0, c.ink(TEXT, 1.0), BOLD);
        txt(
            c,
            20.0,
            314.0,
            &format!(
                "{} · {} cc · {} · {}",
                v.year,
                v.cc,
                if v.color.trim().is_empty() { "sin color" } else { v.color.trim() },
                if v.placa.trim().is_empty() { "sin placa" } else { v.placa.trim() }
            ),
            9.2,
            c.ink(MUTED, 1.0),
            MED,
        );

        // ---- pestañas internas
        let tab_r = c.rect(20.0, 344.0, 360.0, 34.0);
        c.p.rect_filled(tab_r, Rounding::same(c.px(17.0)), c.ink(SURF2, 1.0));
        let nombres = ["Resumen", "Técnico", "Gastos", "Fotos"];
        self.tab_anim += (self.detalle_tab as f32 - self.tab_anim) * 0.25;
        let pill_x = 24.0 + self.tab_anim * 88.0;
        c.p.rect_filled(
            c.rect(pill_x, 348.0, 84.0, 26.0),
            Rounding::same(c.px(13.0)),
            c.ink(ACCENT, 0.18),
        );
        for (k, nombre) in nombres.iter().enumerate() {
            let x = 24.0 + k as f32 * 88.0;
            let hit = c.rect(x, 346.0, 84.0, 30.0);
            let r = ui.interact(hit, Id::new(("dtab", k)), Sense::click());
            txt_c(
                c,
                x + 42.0,
                361.0,
                nombre,
                8.8,
                c.ink(if self.detalle_tab == k { TEXT } else { MUTED }, 1.0),
                if self.detalle_tab == k { SEMI } else { MED },
            );
            if r.clicked() && self.detalle_tab != k {
                self.detalle_tab = k;
                self.tab_anim = k as f32 - 0.6;
            }
        }

        let fila = |c: &Canvas, yy: f32, k: &str, val: &str| {
            txt(c, 36.0, yy, k, 9.8, c.ink(MUTED, 1.0), REG);
            txt_r(c, 364.0, yy - 1.0, val, 10.4, c.ink(TEXT, 0.95), SEMI);
        };
        let seccion = |c: &Canvas, yy: f32, titulo: &str| {
            txt(c, 20.0, yy, titulo, 8.6, c.ink(MUTED, 1.0), SEMI);
        };

        let docs: Vec<Documento> = self.datos.docs.iter().filter(|d| d.vehiculo == i).cloned().collect();

        match self.detalle_tab {
            0 => {
                let est = self.datos.estado().unwrap_or(1.0);
                let tiles = [
                    ("Kilometraje", km(v.km)),
                    ("Estado general", format!("{} %", (est * 100.0) as i32)),
                    ("Placa", if v.placa.trim().is_empty() { "—".into() } else { v.placa.clone() }),
                    ("Año", format!("{}", v.year)),
                ];
                for (k, (label, valor)) in tiles.iter().enumerate() {
                    let col = (k % 2) as f32;
                    let row = (k / 2) as f32;
                    let x = 20.0 + col * 186.0;
                    let yy = 392.0 + row * 70.0;
                    let r = c.rect(x, yy, 174.0, 62.0);
                    card(c, r, 14.0, c.ink(SURF, 1.0), 1.0);
                    txt(c, x + 14.0, yy + 11.0, label, 8.4, c.ink(MUTED, 1.0), SEMI);
                    txt(c, x + 14.0, yy + 28.0, valor, 13.5, c.ink(TEXT, 1.0), SEMI);
                }

                seccion(c, 540.0, "INFORMACIÓN GENERAL");
                let r = c.rect(20.0, 556.0, 360.0, 96.0);
                card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
                let info = [
                    ("Marca", if v.marca.trim().is_empty() { "—".into() } else { v.marca.clone() }),
                    ("Modelo", if v.modelo.trim().is_empty() { "—".into() } else { v.modelo.clone() }),
                    ("Color", if v.color.trim().is_empty() { "—".into() } else { v.color.clone() }),
                    ("Propietario", self.datos.perfil.nombre_o()),
                ];
                for (k, (kk, vv)) in info.iter().enumerate() {
                    let yy = 568.0 + k as f32 * 24.0;
                    fila(c, yy, kk, vv);
                    if k < 3 {
                        divider(c, 36.0, 364.0, yy + 18.0, 0.5);
                    }
                }

                seccion(c, 668.0, "DOCUMENTACIÓN");
                if docs.is_empty() {
                    let r = c.rect(20.0, 684.0, 360.0, 44.0);
                    card(c, r, 16.0, c.ink(SURF, 0.6), 1.0);
                    txt(c, 36.0, 698.0, "Sin documentos registrados", 10.4, c.ink(MUTED, 1.0), MED);
                } else {
                    let r = c.rect(20.0, 684.0, 360.0, docs.len() as f32 * 42.0);
                    card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
                    for (k, d) in docs.iter().enumerate() {
                        let yy = 684.0 + k as f32 * 42.0;
                        let col = if d.vigente() { GREEN } else { AMBER };
                        c.p.circle_filled(c.pos(40.0, yy + 21.0), c.px(11.0), c.ink(col, 0.16));
                        icon(c, Ic::Check, c.pos(40.0, yy + 21.0), c.px(11.0), c.ink(col, 1.0));
                        txt(c, 60.0, yy + 9.0, &d.nombre, 10.8, c.ink(TEXT, 0.95), SEMI);
                        txt(c, 60.0, yy + 25.0, &format!("Vence {}", d.vence), 8.6, c.ink(MUTED, 1.0), REG);
                        txt_r(
                            c,
                            364.0,
                            yy + 13.0,
                            if d.vigente() { "Vigente" } else { "Vencido" },
                            9.2,
                            c.ink(col, 1.0),
                            SEMI,
                        );
                        if k + 1 < docs.len() {
                            divider(c, 60.0, 364.0, yy + 40.0, 0.5);
                        }
                    }
                }
            }
            1 => {
                seccion(c, 392.0, "DATOS TÉCNICOS");
                let r = c.rect(20.0, 408.0, 360.0, 96.0);
                card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
                let consumo = self.datos.consumo();
                let tec = [
                    ("Motor", if v.cc > 0 { format!("{} cc", v.cc) } else { "—".to_string() }),
                    ("Placa", if v.placa.trim().is_empty() { "—".into() } else { v.placa.clone() }),
                    ("Color", if v.color.trim().is_empty() { "—".into() } else { v.color.clone() }),
                    ("Consumo", if consumo > 0 { format!("{consumo} km/gal") } else { "—".to_string() }),
                ];
                for (k, (kk, vv)) in tec.iter().enumerate() {
                    let yy = 420.0 + k as f32 * 24.0;
                    fila(c, yy, kk, vv);
                    if k < 3 {
                        divider(c, 36.0, 364.0, yy + 18.0, 0.5);
                    }
                }

                seccion(c, 520.0, "KILOMETRAJE");
                let r = c.rect(20.0, 536.0, 360.0, 96.0);
                card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
                let ultimo = self
                    .datos
                    .tanqueos
                    .iter()
                    .filter(|t| t.vehiculo == i)
                    .max_by_key(|t| datos::parse_fecha(&t.fecha).map(|(a, b, c)| a * 10000 + b as i32 * 100 + c as i32).unwrap_or(0));
                let kms = [
                    ("Total recorrido", km(v.km)),
                    ("Gasto del mes", money(self.datos.gasto_mes())),
                    (
                        "Último tanqueo",
                        ultimo.map(|t| format!("{} · {}", t.fecha, km(t.km))).unwrap_or("—".to_string()),
                    ),
                    (
                        "Rinde $10.000",
                        if self.datos.km_por_10mil() > 0 {
                            km(self.datos.km_por_10mil())
                        } else {
                            "—".to_string()
                        },
                    ),
                ];
                for (k, (kk, vv)) in kms.iter().enumerate() {
                    let yy = 548.0 + k as f32 * 24.0;
                    fila(c, yy, kk, vv);
                    if k < 3 {
                        divider(c, 36.0, 364.0, yy + 18.0, 0.5);
                    }
                }

                seccion(c, 648.0, "SERVICIOS");
                let servs: Vec<Servicio> = self.datos.servicios.iter().filter(|s| s.vehiculo == i).cloned().collect();
                if servs.is_empty() {
                    let r = c.rect(20.0, 664.0, 360.0, 44.0);
                    card(c, r, 16.0, c.ink(SURF, 0.6), 1.0);
                    txt(c, 36.0, 678.0, "Sin servicios registrados", 10.4, c.ink(MUTED, 1.0), MED);
                } else {
                    let r = c.rect(20.0, 664.0, 360.0, (servs.len() as f32 * 30.0).min(96.0));
                    card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
                    for (k, s) in servs.iter().take(3).enumerate() {
                        let yy = 676.0 + k as f32 * 30.0;
                        txt(c, 36.0, yy, &s.nombre, 9.8, c.ink(TEXT, 0.92), MED);
                        let faltan = s.faltan(km_actual);
                        txt_r(
                            c,
                            364.0,
                            yy,
                            &if faltan > 0 { format!("Faltan {}", km(faltan)) } else { "Vencido".to_string() },
                            9.2,
                            c.ink(if faltan > 0 { MUTED } else { ACCENT }, 1.0),
                            SEMI,
                        );
                        if k + 1 < servs.len().min(3) {
                            divider(c, 36.0, 364.0, yy + 22.0, 0.5);
                        }
                    }
                }
            }
            2 => {
                let r = c.rect(20.0, 392.0, 360.0, 92.0);
                card(c, r, 18.0, c.ink(SURF, 1.0), 1.0);
                grad_v(
                    c,
                    c.rect(20.0, 392.0, 360.0, 92.0),
                    a(c.ink(ACCENT, 1.0), 0.12),
                    a(c.ink(SURF, 1.0), 0.0),
                );
                txt(c, 36.0, 404.0, "GASTO DEL MES", 8.6, c.ink(MUTED, 1.0), SEMI);
                txt(c, 36.0, 418.0, &money(self.datos.gasto_mes()), 24.0, c.ink(TEXT, 1.0), NUM);
                txt_r(c, 364.0, 420.0, "COP", 8.6, c.ink(DIM, 1.0), SEMI);
                txt(c, 36.0, 454.0, &format!("Total histórico {}", money(self.datos.total_tanqueos())), 8.8, c.ink(MUTED, 1.0), REG);

                seccion(c, 502.0, "ÚLTIMOS MOVIMIENTOS");
                let movs = self.datos.movimientos();
                if movs.is_empty() {
                    let r = c.rect(20.0, 518.0, 360.0, 44.0);
                    card(c, r, 16.0, c.ink(SURF, 0.6), 1.0);
                    txt(c, 36.0, 532.0, "Sin movimientos todavía", 10.4, c.ink(MUTED, 1.0), MED);
                } else {
                    let r = c.rect(20.0, 518.0, 360.0, movs.len().min(4) as f32 * 42.0);
                    card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
                    for (k, m) in movs.iter().take(4).enumerate() {
                        let yy = 518.0 + k as f32 * 42.0;
                        let ic = if m.tipo == TipoMov::Tanqueo { Ic::Fuel } else { Ic::Wrench };
                        c.p.circle_filled(c.pos(40.0, yy + 21.0), c.px(11.0), c.ink(SURF3, 0.9));
                        icon(c, ic, c.pos(40.0, yy + 21.0), c.px(11.0), c.ink(MUTED, 1.0));
                        txt(c, 60.0, yy + 9.0, &m.titulo, 9.8, c.ink(TEXT, 0.95), MED);
                        txt(c, 60.0, yy + 25.0, &m.detalle, 8.4, c.ink(MUTED, 1.0), REG);
                        txt_r(c, 364.0, yy + 13.0, &money(m.valor), 10.0, c.ink(TEXT, 1.0), SEMI);
                        if k + 1 < movs.len().min(4) {
                            divider(c, 60.0, 364.0, yy + 40.0, 0.5);
                        }
                    }
                }

                seccion(c, 702.0, "RESUMEN DE GASTOS");
                let r = c.rect(20.0, 718.0, 360.0, 60.0);
                card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
                let gas = [
                    ("Últimos 12 meses", money(self.datos.gasto_12m())),
                    ("Costo por kilómetro", if self.datos.costo_km() > 0.0 { money(self.datos.costo_km()) } else { "—".to_string() }),
                ];
                for (k, (kk, vv)) in gas.iter().enumerate() {
                    let yy = 730.0 + k as f32 * 24.0;
                    fila(c, yy, kk, vv);
                    if k == 0 {
                        divider(c, 36.0, 364.0, yy + 18.0, 0.5);
                    }
                }
            }
            _ => {
                icon(c, Ic::Photo, c.pos(25.0, 396.0), c.px(11.0), c.ink(MUTED, 1.0));
                txt(c, 36.0, 392.0, "GALERÍA DE LA MOTO", 8.6, c.ink(MUTED, 1.0), SEMI);
                for k in 0..4 {
                    let col = (k % 2) as f32;
                    let row = (k / 2) as f32;
                    let x = 20.0 + col * 186.0;
                    let yy = 410.0 + row * 122.0;
                    let r = c.rect(x, yy, 174.0, 112.0);
                    c.p.rect_filled(r, Rounding::same(c.px(14.0)), c.ink(SURF2, 1.0));
                    if k == 0 {
                        self.pintar_foto(c, r, 14.0);
                    } else {
                        silueta(c, r);
                    }
                }
                txt(c, 20.0, 664.0, "Las fotos se guardan solo en tu teléfono.", 8.8, c.ink(DIM, 1.0), REG);
            }
        }

        // ---- botones fijos
        let etiqueta_del = if self.confirmando("del_veh") {
            "¿Seguro?"
        } else {
            "Eliminar moto"
        };
        if boton(c, ui, c.rect(20.0, 780.0, 174.0, 46.0), "edit_veh", "Editar vehículo", true) {
            editar = true;
        }
        let rd = ui.interact(c.rect(206.0, 780.0, 174.0, 46.0), Id::new("del_veh"), Sense::click());
        c.p.rect_filled(c.rect(206.0, 780.0, 174.0, 46.0), Rounding::same(c.px(14.0)), c.ink(ACCENT, 0.10));
        c.p.rect_stroke(
            c.rect(206.0, 780.0, 174.0, 46.0),
            Rounding::same(c.px(14.0)),
            Stroke::new(1.0, c.ink(ACCENT, 0.4)),
        );
        txt_c(c, 293.0, 803.0, etiqueta_del, 10.0, c.ink(ACCENT, 0.95), SEMI);
        if rd.clicked() {
            eliminar = true;
        }

        // ---- barra superior
        base.p.rect_filled(base.rect(0.0, 0.0, base.cw, 70.0), Rounding::ZERO, base.ink(BG, 1.0));
        divider(base, 0.0, base.cw, 70.0, 1.0);
        let back = base.rect(16.0, 20.0, 40.0, 40.0);
        let rb = ui.interact(back, Id::new("back"), Sense::click());
        base.p.rect_filled(back, Rounding::same(base.px(20.0)), base.ink(SURF, 1.0));
        icon(base, Ic::Back, base.pos(36.0, 40.0), base.px(16.0), base.ink(TEXT, 0.95));
        txt_c(base, 200.0, 40.0, "Ficha del vehículo", 12.0, base.ink(TEXT, 0.95), SEMI);
        let gear = base.rect(344.0, 20.0, 40.0, 40.0);
        let rg = ui.interact(gear, Id::new("gear"), Sense::click());
        base.p.rect_filled(gear, Rounding::same(base.px(20.0)), base.ink(SURF, 1.0));
        icon(base, Ic::Chart, base.pos(364.0, 40.0), base.px(15.0), base.ink(MUTED, 1.0));
        if rb.clicked() {
            volver = true;
        }
        if rg.clicked() {
            ir_stats = true;
        }

        if volver {
            self.ir(Pantalla::Principal);
        } else if editar {
            self.abrir_form(FormKind::VehiculoEditar(i));
        } else if ir_stats {
            self.tab = Tab::Stats;
            self.ir(Pantalla::Principal);
        } else if eliminar && self.confirmar("del_veh") {
            self.datos.vehiculos.remove(i);
            if self.datos.sel >= self.datos.vehiculos.len() {
                self.datos.sel = self.datos.vehiculos.len().saturating_sub(1);
            }
            self.guardar();
            if self.datos.vehiculos.is_empty() {
                self.ir(Pantalla::Onboarding);
            } else {
                self.ir(Pantalla::Principal);
            }
            self.avisar("Moto eliminada");
        }
    }

    // ============================================================= formularios
    fn formulario(&mut self, base: &Canvas, ui: &mut egui::Ui, k: FormKind) {
        let e = ease_out(((self.clock - self.entrada) / 0.35) as f32);
        let c = &base.with(e, (1.0 - e) * 14.0);

        let (titulo, sub) = match k {
            FormKind::VehiculoNuevo => ("Nuevo vehículo", "Registra una moto en tu garaje"),
            FormKind::VehiculoEditar(_) => ("Editar vehículo", "Actualiza los datos de tu moto"),
            FormKind::Tanqueo => ("Nuevo tanqueo", "Registra una carga de combustible"),
            FormKind::ServicioNuevo => ("Nuevo servicio", "Programa un mantenimiento"),
            FormKind::ServicioEditar(_) => ("Editar servicio", "Actualiza el mantenimiento"),
            FormKind::Documento => ("Nuevo documento", "SOAT, tecnomecánica y más"),
            FormKind::DocumentoEditar(_) => ("Editar documento", "Actualiza los datos del documento"),
            FormKind::Perfil => ("Editar perfil", "Cómo te llamas y dónde estás"),
            FormKind::Precio => ("Precio del galón", "Cuánto pagas por galón hoy"),
        };

        let y0 = 100.0;
        let mut eliminar_doc = false;
        let mut eliminar_serv = false;
        match k {
            FormKind::VehiculoNuevo | FormKind::VehiculoEditar(_) => {
                self.campos_vehiculo(c, ui, y0);
            }
            FormKind::Tanqueo => {
                self.campos_tanqueo(c, ui, y0);
            }
            FormKind::ServicioNuevo => {
                self.campos_servicio(c, ui, y0);
            }
            FormKind::ServicioEditar(_) => {
                self.campos_servicio(c, ui, y0);
                let r_del = c.rect(20.0, y0 + 340.0, 360.0, 42.0);
                let etiqueta = if self.confirmando("del_serv_form") {
                    "¿Seguro? Toca otra vez"
                } else {
                    "Eliminar servicio"
                };
                if boton(c, ui, r_del, "del_serv_btn", etiqueta, false) {
                    eliminar_serv = true;
                }
            }
            FormKind::Documento => {
                self.campos_documento(c, ui, y0);
            }
            FormKind::DocumentoEditar(_) => {
                self.campos_documento(c, ui, y0);
                let r_del = c.rect(20.0, y0 + 260.0, 360.0, 42.0);
                let etiqueta = if self.confirmando("del_doc_form") {
                    "¿Seguro? Toca otra vez"
                } else {
                    "Eliminar documento"
                };
                if boton(c, ui, r_del, "del_doc_btn", etiqueta, false) {
                    eliminar_doc = true;
                }
            }
            FormKind::Perfil => {
                self.campos_perfil(c, ui, y0);
            }
            FormKind::Precio => {
                self.campos_precio(c, ui, y0);
            }
        }

        if eliminar_doc && self.confirmar("del_doc_form") {
            if let FormKind::DocumentoEditar(i) = k {
                let v = self.datos.indice();
                let docs_indices: Vec<usize> = self.datos.docs.iter().enumerate()
                    .filter(|(_, d)| d.vehiculo == v)
                    .map(|(k, _)| k)
                    .collect();
                if let Some(&doc_idx) = docs_indices.get(i) {
                    if doc_idx < self.datos.docs.len() {
                        self.datos.docs.remove(doc_idx);
                        self.guardar();
                        self.avisar("Documento eliminado");
                        self.cerrar_form();
                        return;
                    }
                }
            }
        }
        if eliminar_serv && self.confirmar("del_serv_form") {
            if let FormKind::ServicioEditar(i) = k {
                if i < self.datos.servicios.len() {
                    self.datos.servicios.remove(i);
                    self.guardar();
                    self.avisar("Servicio eliminado");
                    self.cerrar_form();
                    return;
                }
            }
        }

        // ---- barra superior
        base.p.rect_filled(
            base.rect(0.0, 0.0, base.cw, 76.0),
            Rounding::ZERO,
            base.ink(BG, 1.0),
        );
        divider(base, 0.0, base.cw, 76.0, 1.0);
        let back = base.rect(16.0, 20.0, 40.0, 40.0);
        let rb = ui.interact(back, Id::new("form_back"), Sense::click());
        base.p.rect_filled(back, Rounding::same(base.px(20.0)), base.ink(SURF, 1.0));
        icon(base, Ic::Back, base.pos(36.0, 40.0), base.px(16.0), base.ink(TEXT, 0.95));
        txt(base, 68.0, 20.0, titulo, 15.0, base.ink(TEXT, 1.0), BOLD);
        txt(base, 68.0, 42.0, sub, 9.2, base.ink(MUTED, 1.0), MED);

        // ---- botón fijo o teclado en pantalla
        let mut guardar = false;
        if self.foco.is_some() && !cfg!(target_os = "android") {
            self.teclado(c, ui);
        } else if boton(c, ui, c.rect(20.0, self.lay.bh - 74.0, 360.0, 50.0), "form_ok", "Guardar", true) {
            guardar = true;
        }

        if guardar {
            self.guardar_form(k);
        } else if rb.clicked() {
            self.cerrar_form();
        }
    }

    // ========================================================= caja inteligente
    fn caja(&mut self, base: &Canvas, ui: &mut egui::Ui) {
        let e = ease_out(((self.clock - self.entrada) / 0.35) as f32);
        let c = &base.with(e, (1.0 - e) * 12.0);

        // Barra superior
        base.p.rect_filled(
            base.rect(0.0, 0.0, base.cw, 76.0),
            Rounding::ZERO,
            base.ink(BG, 1.0),
        );
        divider(base, 0.0, base.cw, 76.0, 1.0);
        let back = base.rect(16.0, 20.0, 40.0, 40.0);
        let rb = ui.interact(back, Id::new("caja_back"), Sense::click());
        base.p.rect_filled(back, Rounding::same(base.px(20.0)), base.ink(SURF, 1.0));
        icon(base, Ic::Back, base.pos(36.0, 40.0), base.px(16.0), base.ink(TEXT, 0.95));
        txt(base, 68.0, 20.0, "Caja Inteligente", 15.5, base.ink(TEXT, 1.0), BOLD);
        txt(base, 68.0, 42.0, "Registro rápido por voz o texto · 100% offline", 9.2, base.ink(MUTED, 1.0), MED);

        let mut scroll = 0.0f32;
        let alto = 750.0;
        let cs = &scrollable(c, ui, &mut scroll, alto, 76.0, 780.0);

        let mut y = 92.0;

        // Tarjeta principal con el botón de micrófono
        let mic_card = cs.rect(20.0, y, 360.0, 120.0);
        card(cs, mic_card, 18.0, cs.ink(SURF, 1.0), 1.0);
        grad_v(cs, mic_card, a(cs.ink(ACCENT, 1.0), 0.12), a(cs.ink(SURF, 1.0), 0.0));
        cs.p.rect_stroke(mic_card, Rounding::same(cs.px(18.0)), Stroke::new(1.0, cs.ink(ACCENT, 0.3)));

        let mic_center = cs.pos(74.0, y + 60.0);
        let mic_btn = cs.rect(40.0, y + 26.0, 68.0, 68.0);
        let resp_mic = ui.interact(mic_btn, Id::new("caja_mic_btn"), Sense::click());

        // Pulso animado si está escuchando o en reposo
        let pulso = (self.clock * 3.0).sin().abs() as f32;
        let ring_rad = cs.px(34.0 + if self.escuchando { pulso * 8.0 } else { pulso * 2.5 });
        cs.p.circle_stroke(mic_center, ring_rad, Stroke::new(1.4, cs.ink(ACCENT, if self.escuchando { 0.8 } else { 0.3 })));
        cs.p.circle_filled(mic_center, cs.px(30.0), cs.ink(if self.escuchando { ACCENT } else { SURF2 }, 1.0));
        cs.p.circle_stroke(mic_center, cs.px(30.0), Stroke::new(1.2, cs.ink(ACCENT, 0.9)));
        icon(cs, Ic::Bolt, mic_center, cs.px(22.0), cs.ink(if self.escuchando { Color32::WHITE } else { ACCENT }, 1.0));

        txt(cs, 124.0, y + 36.0, if self.escuchando { "Escuchando tu voz…" } else { "Habla o escribe tu registro" }, 12.0, cs.ink(TEXT, 1.0), BOLD);
        txt(cs, 124.0, y + 58.0, "Dile a la app qué hiciste con tu moto", 9.2, cs.ink(MUTED, 1.0), REG);
        txt(cs, 124.0, y + 74.0, "Sin nube · se procesa en el teléfono", 8.4, cs.ink(GREEN, 1.0), SEMI);

        #[cfg(target_os = "android")]
        if resp_mic.clicked() {
            self.escuchando = !self.escuchando;
            if self.escuchando {
                ime::escuchar();
            }
        }
        #[cfg(not(target_os = "android"))]
        if resp_mic.clicked() {
            self.avisar("En escritorio escribe en el campo inferior");
        }

        y += 134.0;

        // Campo de texto
        etiqueta(cs, 20.0, y, "¿QUÉ HICISTE? (TEXTO O VOZ)");
        campo_txt(
            cs,
            ui,
            cs.rect(20.0, y + 14.0, 360.0, 48.0),
            "caja",
            &mut self.form.caja,
            &mut self.foco,
            "Ej: Tanqueé 50.000 a 16.000 en Terpel",
        );
        y += 72.0;

        // Atajos rápidos / Ejemplos clicables
        txt(cs, 20.0, y, "O PRUEBA CON UN EJEMPLO", 8.6, cs.ink(MUTED, 1.0), SEMI);
        y += 14.0;
        let ejemplos = [
            "Tanqueé 45.000 en Terpel",
            "Cambio de aceite a 15.000 km",
            "SOAT vence en 1 año",
            "Tecnomecánica el 10/11/2027",
        ];
        for (idx, ej) in ejemplos.iter().enumerate() {
            let row = (idx / 2) as f32;
            let col = (idx % 2) as f32;
            let bx = 20.0 + col * 186.0;
            let by = y + row * 40.0;
            let r = cs.rect(bx, by, 174.0, 34.0);
            if boton(cs, ui, r, &format!("ej_{idx}"), ej, false) {
                self.form.caja = ej.to_string();
            }
        }
        y += 88.0;

        // Detección en tiempo real de NLP
        let texto_actual = self.form.caja.trim();
        let idea = if !texto_actual.is_empty() {
            entender::interpretar(texto_actual, datos::hoy())
        } else {
            entender::Idea::Nada
        };

        txt(cs, 20.0, y, "ANÁLISIS INTELIGENTE", 8.6, cs.ink(MUTED, 1.0), SEMI);
        y += 14.0;
        let card_pred = cs.rect(20.0, y, 360.0, 92.0);
        card(cs, card_pred, 16.0, cs.ink(SURF, 1.0), 1.0);

        match &idea {
            entender::Idea::Tanqueo { monto, precio, km: k_km, estacion } => {
                cs.p.rect_stroke(card_pred, Rounding::same(cs.px(16.0)), Stroke::new(1.2, cs.ink(GREEN, 0.8)));
                chip(cs, 36.0, y + 14.0, 96.0, 18.0, "TANQUEO", GREEN, 0.18);
                let monto_str = monto.map(money).unwrap_or_else(|| "Monto sin definir".into());
                txt(cs, 36.0, y + 38.0, &monto_str, 15.0, cs.ink(TEXT, 1.0), BOLD);
                let est_txt = if estacion.is_empty() { "Estación automática" } else { estacion.as_str() };
                let pr_txt = precio.map(|p| format!("a {}", money(p))).unwrap_or_default();
                let km_txt = k_km.map(km).unwrap_or_default();
                txt(cs, 36.0, y + 62.0, &format!("{est_txt} {pr_txt} {km_txt}"), 9.4, cs.ink(MUTED, 1.0), MED);
            }
            entender::Idea::Servicio { nombre, cada_km, costo, .. } => {
                cs.p.rect_stroke(card_pred, Rounding::same(cs.px(16.0)), Stroke::new(1.2, cs.ink(ACCENT, 0.8)));
                chip(cs, 36.0, y + 14.0, 96.0, 18.0, "SERVICIO", ACCENT, 0.18);
                txt(cs, 36.0, y + 38.0, nombre, 14.0, cs.ink(TEXT, 1.0), BOLD);
                let cada_str = cada_km.map(|k| format!("cada {}", km(k))).unwrap_or_else(|| "cada 3.000 km".into());
                let costo_str = costo.map(money).unwrap_or_default();
                txt(cs, 36.0, y + 62.0, &format!("{cada_str} · {costo_str}"), 9.4, cs.ink(MUTED, 1.0), MED);
            }
            entender::Idea::Documento { nombre, vence } => {
                cs.p.rect_stroke(card_pred, Rounding::same(cs.px(16.0)), Stroke::new(1.2, cs.ink(AMBER, 0.8)));
                chip(cs, 36.0, y + 14.0, 100.0, 18.0, "DOCUMENTO", AMBER, 0.18);
                txt(cs, 36.0, y + 38.0, nombre, 14.0, cs.ink(TEXT, 1.0), BOLD);
                let v_str = vence.clone().unwrap_or_else(|| "en 1 año".into());
                txt(cs, 36.0, y + 62.0, &format!("Vencimiento estimado: {v_str}"), 9.4, cs.ink(MUTED, 1.0), MED);
            }
            entender::Idea::Nada => {
                if texto_actual.is_empty() {
                    txt(cs, 36.0, y + 32.0, "Escribe o dicta para que la app lo interprete", 10.4, cs.ink(MUTED, 1.0), MED);
                    txt(cs, 36.0, y + 54.0, "Detecta tanqueos, servicios y vencimientos automáticamente", 8.8, cs.ink(DIM, 1.0), REG);
                } else {
                    txt(cs, 36.0, y + 32.0, "Frase no reconocida todavía", 10.4, cs.ink(AMBER, 1.0), MED);
                    txt(cs, 36.0, y + 54.0, "Prueba escribiendo «tanqueé 40.000» o «cambio de aceite»", 8.8, cs.ink(MUTED, 1.0), REG);
                }
            }
        }
        y += 108.0;

        // Estado anterior si existe
        if !self.caja_estado.is_empty() {
            let r_est = cs.rect(20.0, y, 360.0, 42.0);
            card(cs, r_est, 12.0, cs.ink(SURF2, 0.8), 1.0);
            icon(cs, Ic::Check, cs.pos(38.0, y + 21.0), cs.px(12.0), cs.ink(GREEN, 1.0));
            txt(cs, 56.0, y + 14.0, &self.caja_estado, 9.6, cs.ink(TEXT, 0.95), MED);
        }

        // Botón procesar o teclado
        if self.foco.is_some() && !cfg!(target_os = "android") {
            self.teclado(base, ui);
        } else {
            let es_valido = !matches!(idea, entender::Idea::Nada);
            if boton(base, ui, base.rect(20.0, self.lay.bh - 74.0, 360.0, 50.0), "caja_guardar", "Procesar y Guardar", es_valido) {
                self.interpretar_caja();
            }
        }

        if rb.clicked() {
            self.foco = None;
            self.ir(self.volver.clone());
        }
    }

    /// Inserta texto en el campo con foco (lo usan teclado propio y físico).
    fn insertar(&mut self, texto: &str) {
        if let Some(id) = self.foco.clone() {
            // Los campos de fecha solo aceptan dígitos y las barras salen solas.
            if es_fecha(&id) {
                let reemplazar = self.foco_prev.as_deref() != Some(id.as_str());
                let actual = self.form.buf_mut(&id).map(|b| b.clone()).unwrap_or_default();
                let base = if reemplazar { String::new() } else { actual };
                let mut digitos: String = base.chars().filter(|c| c.is_ascii_digit()).collect();
                for c in texto.chars() {
                    if c.is_ascii_digit() && digitos.len() < 8 {
                        digitos.push(c);
                    }
                }
                if let Some(b) = self.form.buf_mut(&id) {
                    *b = formatear_fecha(&digitos);
                }
                return;
            }
            if let Some(b) = self.form.buf_mut(&id) {
                b.push_str(texto);
            }
        }
    }

    /// Borra el último carácter del campo con foco.
    fn borrar_char(&mut self) {
        if let Some(id) = self.foco.clone() {
            if es_fecha(&id) {
                let actual = self.form.buf_mut(&id).map(|b| b.clone()).unwrap_or_default();
                let mut digitos: String = actual.chars().filter(|c| c.is_ascii_digit()).collect();
                digitos.pop();
                if let Some(b) = self.form.buf_mut(&id) {
                    *b = formatear_fecha(&digitos);
                }
                return;
            }
            if let Some(b) = self.form.buf_mut(&id) {
                b.pop();
            }
        }
    }

    /// Atiende el teclado del sistema Android: aplica lo que se escribió en el
    /// campo activo y muestra u oculta el IME según el foco.
    #[cfg(target_os = "android")]
    fn ime_android(&mut self) {
        for ev in ime::drenar() {
            match ev {
                ime::Evento::Texto(t) => self.insertar(&t),
                ime::Evento::Borrar => self.borrar_char(),
                ime::Evento::Listo => self.foco = None,
                ime::Evento::Voz(t) => {
                    self.escuchando = false;
                    if !matches!(self.pantalla, Pantalla::Caja) {
                        self.abrir_caja();
                    }
                    self.form.caja = t;
                    self.interpretar_caja();
                }
                ime::Evento::VozError(e) => {
                    self.escuchando = false;
                    self.avisar(&e);
                }
            }
        }

        let clics = CLICS_CAMPO.load(Ordering::Relaxed);
        match self.foco.clone() {
            Some(id) => {
                if self.foco_prev.as_deref() != Some(id.as_str())
                    || clics != self.clics_prev
                    || !self.ime_abierto
                {
                    ime::mostrar(teclado_para(&id));
                    self.ime_abierto = true;
                }
            }
            None => {
                if self.ime_abierto {
                    ime::ocultar();
                    self.ime_abierto = false;
                }
            }
        }
        self.foco_prev = self.foco.clone();
        self.clics_prev = clics;
    }

    /// Teclado en pantalla. Se dibuja sobre la mitad inferior del formulario y
    /// evita depender del IME del sistema (que en Android no entrega el texto
    /// táctil a winit). Soporta acentos y ñ, más una capa numérica.
    fn teclado(&mut self, c: &Canvas, ui: &mut egui::Ui) {
        // --- teclado físico (escritorio; en Android llegan las teclas ASCII)
        let eventos = ui.input(|i| i.events.clone());
        for ev in &eventos {
            match ev {
                egui::Event::Text(t) => self.insertar(t),
                egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                } => self.borrar_char(),
                egui::Event::Key {
                    key: egui::Key::Enter | egui::Key::Escape,
                    pressed: true,
                    ..
                } => {
                    self.foco = None;
                    return;
                }
                _ => {}
            }
        }

        // Si cambió el campo enfocado, elegí la capa que corresponda.
        if self.foco != self.foco_prev {
            self.foco_prev = self.foco.clone();
            self.mayus = false;
            if let Some(id) = self.foco.clone() {
                self.capa = if es_numerico(&id) {
                    Capa::Numeros
                } else {
                    Capa::Letras
                };
            }
        }

        // --- panel
        let panel = c.rect(0.0, 500.0, BASE_W, 366.0);
        c.p.rect_filled(panel, Rounding::ZERO, c.ink(SURF2, 1.0));
        divider(c, 0.0, BASE_W, 500.0, 1.4);

        txt(
            c,
            16.0,
            512.0,
            "Toca un campo y escribe",
            8.6,
            c.ink(MUTED, 1.0),
            MED,
        );
        let r_listo = c.rect(BASE_W - 94.0, 505.0, 80.0, 34.0);
        if tecla_kb(c, ui, r_listo, "Listo", "kb_listo", true, true, 11.5) {
            self.foco = None;
            return;
        }

        // --- distribución de filas
        let letras = |s: &str| -> Vec<(f32, Tecla)> {
            s.chars().map(|ch| (1.0, Tecla::Letra(ch))).collect()
        };
        let simbolos = |s: &[&'static str]| -> Vec<(f32, Tecla)> {
            s.iter().map(|x| (1.0, Tecla::Simbolo(x))).collect()
        };

        let filas: Vec<Vec<(f32, Tecla)>> = match self.capa {
            Capa::Letras => {
                let mut r3 = vec![(1.6, Tecla::Mayus)];
                r3.extend(letras("zxcvbnm"));
                r3.push((1.6, Tecla::Borrar));
                let mut r4 = vec![(1.7, Tecla::Capa(Capa::Numeros))];
                r4.extend(letras("áéíóú"));
                r4.push((4.0, Tecla::Espacio));
                vec![letras("qwertyuiop"), letras("asdfghjklñ"), r3, r4]
            }
            Capa::Numeros => {
                let mut r3 = simbolos(&[".", ",", "#", "+", "*", "=", "%", "!", "?"]);
                r3.push((1.6, Tecla::Borrar));
                let mut r4 = vec![(1.7, Tecla::Capa(Capa::Letras))];
                r4.extend(simbolos(&["'", "\""]));
                r4.push((4.0, Tecla::Espacio));
                vec![letras("1234567890"), simbolos(&["-", "/", ":", ";", "(", ")", "$", "&", "@"]), r3, r4]
            }
        };

        let x0 = 4.0f32;
        let w_total = (BASE_W - 4.0) - x0;
        let gap = 4.0f32;
        let alto = 74.0f32;
        let mut y = 550.0f32;
        let mut contador = 0usize;

        for fila in &filas {
            let n = fila.len() as f32;
            let unidades: f32 = fila.iter().map(|(u, _)| *u).sum();
            let ux = (w_total - gap * (n - 1.0)) / unidades;
            let mut x = x0;
            for (u, t) in fila {
                let w = ux * *u;
                let r = c.rect(x, y, w, alto);
                let id = format!("kb{contador}");
                contador += 1;
                let label = etiqueta_tecla(*t, self.mayus);
                let util = matches!(t, Tecla::Borrar | Tecla::Capa(_));
                let activa = matches!(t, Tecla::Mayus) && self.mayus;
                let size = if label.len() > 3 { 9.0 } else { 14.0 };
                if tecla_kb(c, ui, r, &label, &id, util, activa, size) {
                    match t {
                        Tecla::Letra(ch) => {
                            let s = if self.mayus {
                                ch.to_uppercase().to_string()
                            } else {
                                ch.to_string()
                            };
                            self.insertar(&s);
                            if self.mayus {
                                self.mayus = false;
                            }
                        }
                        Tecla::Simbolo(s) => self.insertar(s),
                        Tecla::Espacio => self.insertar(" "),
                        Tecla::Borrar => self.borrar_char(),
                        Tecla::Mayus => self.mayus = !self.mayus,
                        Tecla::Capa(cp) => {
                            self.capa = *cp;
                            self.mayus = false;
                        }
                    }
                }
                x += w + gap;
            }
            y += alto + 6.0;
        }
    }

    fn campos_vehiculo(&mut self, c: &Canvas, ui: &mut egui::Ui, y0: f32) -> f32 {
        let (x0, w1) = (20.0f32, 360.0f32);
        let (x2, w2) = (206.0f32, 174.0f32);
        let h = 46.0f32;
        let mut y = y0;

        etiqueta(c, x0, y, "MARCA");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w1, h),
            "v_marca",
            &mut self.form.marca,
            &mut self.foco,
            "Yamaha, Honda, Bajaj…",
        );
        y += 66.0;

        etiqueta(c, x0, y, "MODELO / LÍNEA");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w1, h),
            "v_modelo",
            &mut self.form.modelo,
            &mut self.foco,
            "R15, CB 125, Pulsar…",
        );
        y += 66.0;

        etiqueta(c, x0, y, "AÑO");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w2, h),
            "v_year",
            &mut self.form.year,
            &mut self.foco,
            "2019",
            1.0,
            1950.0,
            2100.0,
            Num::Entero,
        );
        etiqueta(c, x2, y, "CILINDRAJE (CC)");
        campo_num(
            c,
            ui,
            c.rect(x2, y + 14.0, w2, h),
            "v_cc",
            &mut self.form.cc,
            &mut self.foco,
            "150",
            10.0,
            50.0,
            2000.0,
            Num::Entero,
        );
        y += 66.0;

        etiqueta(c, x0, y, "KILOMETRAJE DEL TABLERO");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w1, h),
            "v_km",
            &mut self.form.km,
            &mut self.foco,
            "0",
            100.0,
            0.0,
            1_000_000.0,
            Num::Entero,
        );
        y += 66.0;

        etiqueta(c, x0, y, "PLACA");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w2, h),
            "v_placa",
            &mut self.form.placa,
            &mut self.foco,
            "ABC 12D",
        );
        etiqueta(c, x2, y, "COLOR");
        campo_txt(
            c,
            ui,
            c.rect(x2, y + 14.0, w2, h),
            "v_color",
            &mut self.form.color,
            &mut self.foco,
            "Negro, azul…",
        );
        y += 66.0;

        y
    }

    fn campos_tanqueo(&mut self, c: &Canvas, ui: &mut egui::Ui, y0: f32) -> f32 {
        let (x0, w) = (20.0f32, 360.0f32);
        let h = 46.0f32;
        let mut y = y0;

        etiqueta(c, x0, y, "FECHA (DD/MM/AAAA)");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "t_fecha",
            &mut self.form.fecha,
            &mut self.foco,
            "01/01/2026",
        );
        y += 66.0;

        etiqueta(c, x0, y, "KILOMETRAJE DEL TABLERO");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "t_km",
            &mut self.form.km,
            &mut self.foco,
            "0",
            10.0,
            0.0,
            1_000_000.0,
            Num::Entero,
        );
        y += 66.0;

        etiqueta(c, x0, y, "¿CUÁNTO PAGASTE DE GASOLINA? (COP)");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "t_monto",
            &mut self.form.monto,
            &mut self.foco,
            "20000",
            1000.0,
            0.0,
            2_000_000.0,
            Num::Entero,
        );
        y += 66.0;

        etiqueta(c, x0, y, "PRECIO POR GALÓN (COP)");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "t_precio",
            &mut self.form.precio_galon,
            &mut self.foco,
            "13900",
            100.0,
            0.0,
            100_000.0,
            Num::Entero,
        );
        y += 66.0;

        etiqueta(c, x0, y, "ESTACIÓN DE SERVICIO");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "t_est",
            &mut self.form.estacion,
            &mut self.foco,
            "Terpel, Primax, Texaco…",
        );
        y += 66.0;

        // resumen: lo que pagó y a cuántos galones equivale
        let monto = a_dec(&self.form.monto);
        let precio = a_dec(&self.form.precio_galon);
        let gal = if precio > 0.0 { monto / precio } else { 0.0 };
        let r = c.rect(x0, y, w, 70.0);
        card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
        grad_v(
            c,
            r,
            a(c.ink(ACCENT, 1.0), 0.12),
            a(c.ink(SURF, 1.0), 0.0),
        );
        icon(c, Ic::Fuel, c.pos(x0 + 30.0, y + 35.0), c.px(18.0), c.ink(ACCENT, 0.9));
        txt(c, x0 + 52.0, y + 14.0, "Total a pagar", 9.4, c.ink(MUTED, 1.0), MED);
        txt(
            c,
            x0 + 52.0,
            y + 29.0,
            &if monto > 0.0 { money(monto) } else { "$ 0".to_string() },
            18.0,
            c.ink(TEXT, 1.0),
            NUM,
        );
        let equiv = if gal > 0.0 {
            format!("Equivale a {gal:.2} galones a {}", money(precio))
        } else {
            "Escribe el monto y el precio del galón".to_string()
        };
        txt(c, x0 + 52.0, y + 54.0, &equiv, 8.6, c.ink(MUTED, 1.0), REG);
        y += 86.0;

        y
    }

    fn campos_servicio(&mut self, c: &Canvas, ui: &mut egui::Ui, y0: f32) -> f32 {
        let (x0, w) = (20.0f32, 360.0f32);
        let (x2, w2) = (206.0f32, 174.0f32);
        let h = 46.0f32;
        let mut y = y0;

        etiqueta(c, x0, y, "NOMBRE DEL SERVICIO");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "s_nombre",
            &mut self.form.s_nombre,
            &mut self.foco,
            "Cambio de aceite, kit de arrastre…",
        );
        y += 66.0;

        etiqueta(c, x0, y, "DETALLE (OPCIONAL)");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "s_detalle",
            &mut self.form.s_detalle,
            &mut self.foco,
            "10W-40, pastillas delanteras…",
        );
        y += 66.0;

        etiqueta(c, x0, y, "CADA CUÁNTOS KM");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w2, h),
            "s_cada",
            &mut self.form.s_cada,
            &mut self.foco,
            "3000",
            500.0,
            0.0,
            200_000.0,
            Num::Entero,
        );
        etiqueta(c, x2, y, "ÚLTIMO CAMBIO (KM)");
        campo_num(
            c,
            ui,
            c.rect(x2, y + 14.0, w2, h),
            "s_ult",
            &mut self.form.s_ultimo,
            &mut self.foco,
            "0",
            100.0,
            0.0,
            1_000_000.0,
            Num::Entero,
        );
        y += 66.0;

        etiqueta(c, x0, y, "COSTO (COP)");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w2, h),
            "s_costo",
            &mut self.form.s_costo,
            &mut self.foco,
            "0",
            5_000.0,
            0.0,
            100_000_000.0,
            Num::Entero,
        );
        etiqueta(c, x2, y, "FECHA DEL CAMBIO");
        campo_txt(
            c,
            ui,
            c.rect(x2, y + 14.0, w2, h),
            "s_fecha",
            &mut self.form.s_fecha,
            &mut self.foco,
            "dd/mm/aaaa",
        );
        y += 66.0;

        y
    }

    fn campos_documento(&mut self, c: &Canvas, ui: &mut egui::Ui, y0: f32) -> f32 {
        let (x0, w) = (20.0f32, 360.0f32);
        let h = 46.0f32;
        let mut y = y0;

        etiqueta(c, x0, y, "NOMBRE DEL DOCUMENTO");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "d_nombre",
            &mut self.form.d_nombre,
            &mut self.foco,
            "SOAT, Tecnomecánica…",
        );
        y += 66.0;

        etiqueta(c, x0, y, "FECHA DE VENCIMIENTO");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "d_vence",
            &mut self.form.d_vence,
            &mut self.foco,
            "dd/mm/aaaa",
        );
        y += 60.0;

        txt(c, x0, y, "VENCE EN…", 8.4, c.ink(MUTED, 1.0), SEMI);
        y += 14.0;
        let dia = datos::hoy().2;
        let atajos = [
            ("6 meses", datos::fecha_mas_meses(6, dia)),
            ("1 año", datos::fecha_mas_meses(12, dia)),
            ("2 años", datos::fecha_mas_meses(24, dia)),
        ];
        for (i, (label, fecha)) in atajos.iter().enumerate() {
            let bw = 116.0;
            let bx = x0 + i as f32 * 122.0;
            if boton(c, ui, c.rect(bx, y, bw, 40.0), &format!("d_q{i}"), label, false) {
                self.form.d_vence = fecha.clone();
            }
        }
        y += 58.0;

        y
    }

    fn campos_perfil(&mut self, c: &Canvas, ui: &mut egui::Ui, y0: f32) -> f32 {
        let (x0, w) = (20.0f32, 360.0f32);
        let h = 46.0f32;
        let mut y = y0;

        etiqueta(c, x0, y, "NOMBRE COMPLETO");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "p_nombre",
            &mut self.form.p_nombre,
            &mut self.foco,
            "Tu nombre",
        );
        y += 66.0;

        etiqueta(c, x0, y, "CORREO ELECTRÓNICO");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "p_email",
            &mut self.form.p_email,
            &mut self.foco,
            "tucorreo@ejemplo.com",
        );
        y += 66.0;

        etiqueta(c, x0, y, "CIUDAD");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "p_ciudad",
            &mut self.form.p_ciudad,
            &mut self.foco,
            "Bogotá D.C.",
        );
        y += 66.0;

        etiqueta(c, x0, y, "TELÉFONO");
        campo_txt(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "p_tel",
            &mut self.form.p_telefono,
            &mut self.foco,
            "+57 300 000 0000",
        );
        y += 66.0;

        y
    }

    fn campos_precio(&mut self, c: &Canvas, ui: &mut egui::Ui, y0: f32) -> f32 {
        let (x0, w) = (20.0f32, 360.0f32);
        let h = 46.0f32;
        let mut y = y0;

        etiqueta(c, x0, y, "PRECIO POR GALÓN (COP)");
        campo_num(
            c,
            ui,
            c.rect(x0, y + 14.0, w, h),
            "pr_gal",
            &mut self.form.precio_galon,
            &mut self.foco,
            "13900",
            100.0,
            0.0,
            100_000.0,
            Num::Entero,
        );
        y += 66.0;

        let r = c.rect(x0, y, w, 96.0);
        card(c, r, 16.0, c.ink(SURF, 1.0), 1.0);
        icon(c, Ic::Fuel, c.pos(x0 + 30.0, y + 30.0), c.px(18.0), c.ink(ACCENT, 0.9));
        txt(c, x0 + 52.0, y + 16.0, "Precio actual", 9.4, c.ink(MUTED, 1.0), MED);
        let p = a_dec(&self.form.precio_galon);
        txt(
            c,
            x0 + 52.0,
            y + 32.0,
            &if p > 0.0 { money(p) } else { "Sin definir".to_string() },
            20.0,
            c.ink(TEXT, 1.0),
            NUM,
        );
        txt(
            c,
            x0 + 52.0,
            y + 64.0,
            "Se usa para pasar de plata a galones y medir el rendimiento.",
            8.8,
            c.ink(MUTED, 1.0),
            REG,
        );
        y += 112.0;

        y
    }

    fn guardar_form(&mut self, k: FormKind) {
        match k {
            FormKind::VehiculoNuevo => {
                if self.form.marca.trim().is_empty() || self.form.modelo.trim().is_empty() {
                    self.avisar("Escribe la marca y el modelo");
                    return;
                }
                let year = solo_digitos(&self.form.year) as i32;
                let cc = solo_digitos(&self.form.cc) as i32;
                let km = solo_digitos(&self.form.km);
                if !(1950..=datos::hoy().0).contains(&year) {
                    self.avisar("Revisa el año del vehículo");
                    return;
                }
                if km < 0 || cc <= 0 {
                    self.avisar("Revisa el kilometraje y los centímetros cúbicos");
                    return;
                }
                let vacio = self.datos.vehiculos.is_empty();
                self.datos.vehiculos.push(Vehiculo {
                    marca: self.form.marca.trim().to_string(),
                    modelo: self.form.modelo.trim().to_string(),
                    year,
                    km,
                    placa: self.form.placa.trim().to_uppercase(),
                    color: self.form.color.trim().to_string(),
                    cc: if cc > 0 { cc } else { 150 },
                    principal: vacio,
                });
                self.datos.sel = self.datos.vehiculos.len() - 1;
            }
            FormKind::VehiculoEditar(i) => {
                let year = solo_digitos(&self.form.year) as i32;
                let cc = solo_digitos(&self.form.cc) as i32;
                let km = solo_digitos(&self.form.km);
                if !(1950..=datos::hoy().0).contains(&year) {
                    self.avisar("Revisa el año del vehículo");
                    return;
                }
                if km < 0 || cc <= 0 {
                    self.avisar("Revisa el kilometraje y los centímetros cúbicos");
                    return;
                }
                if let Some(v) = self.datos.vehiculos.get_mut(i) {
                    v.marca = self.form.marca.trim().to_string();
                    v.modelo = self.form.modelo.trim().to_string();
                    v.year = year;
                    v.km = km;
                    v.placa = self.form.placa.trim().to_uppercase();
                    v.color = self.form.color.trim().to_string();
                    v.cc = cc;
                }
                self.datos.sel = i.min(self.datos.vehiculos.len().saturating_sub(1));
            }
            FormKind::Tanqueo => {
                let monto = a_dec(&self.form.monto);
                let precio = a_dec(&self.form.precio_galon);
                let km = solo_digitos(&self.form.km);
                if monto <= 0.0 {
                    self.avisar("Escribe cuánto pagaste de gasolina");
                    return;
                }
                if precio <= 0.0 {
                    self.avisar("Escribe el precio por galón");
                    return;
                }
                if km < 0 {
                    self.avisar("Escribe el kilometraje del tablero");
                    return;
                }
                let km_actual = self.datos.km_actual();
                if km < km_actual {
                    self.avisar("El kilometraje no puede ser menor al actual");
                    return;
                }
                if datos::parse_fecha(&self.form.fecha).is_none() {
                    self.avisar("Escribe una fecha válida");
                    return;
                }
                let v = self.datos.indice();
                let fecha = self.form.fecha.trim().to_string();
                self.datos.tanqueos.push(Tanqueo {
                    vehiculo: v,
                    fecha,
                    estacion: self.form.estacion.trim().to_string(),
                    monto,
                    galones: 0.0,
                    precio_galon: precio,
                    km,
                });
                self.datos.precio_galon = precio;
                if let Some(ve) = self.datos.vehiculos.get_mut(v) {
                    if km > ve.km {
                        ve.km = km;
                    }
                }
            }
            FormKind::ServicioNuevo => {
                if self.form.s_nombre.trim().is_empty() {
                    self.avisar("Escribe el nombre del servicio");
                    return;
                }
                let v = self.datos.indice();
                let cada = solo_digitos(&self.form.s_cada);
                let ultimo = solo_digitos(&self.form.s_ultimo);
                let km_actual = self.datos.km_actual();
                if cada <= 0 || ultimo < 0 {
                    self.avisar("Revisa el intervalo y el último kilometraje");
                    return;
                }
                self.datos.servicios.push(Servicio {
                    vehiculo: v,
                    nombre: self.form.s_nombre.trim().to_string(),
                    detalle: self.form.s_detalle.trim().to_string(),
                    cada_km: if cada > 0 { cada } else { 3000 },
                    ultimo_km: if ultimo > 0 { ultimo } else { km_actual },
                    costo: a_dec(&self.form.s_costo),
                    ultima_fecha: self.form.s_fecha.trim().to_string(),
                });
            }
            FormKind::ServicioEditar(i) => {
                let cada = solo_digitos(&self.form.s_cada);
                let ultimo = solo_digitos(&self.form.s_ultimo);
                if self.form.s_nombre.trim().is_empty() {
                    self.avisar("Escribe el nombre del servicio");
                    return;
                }
                if cada <= 0 || ultimo < 0 {
                    self.avisar("Revisa el intervalo y el último kilometraje");
                    return;
                }
                if let Some(s) = self.datos.servicios.get_mut(i) {
                    s.nombre = self.form.s_nombre.trim().to_string();
                    s.detalle = self.form.s_detalle.trim().to_string();
                    s.cada_km = cada;
                    s.ultimo_km = ultimo;
                    s.costo = a_dec(&self.form.s_costo);
                    s.ultima_fecha = self.form.s_fecha.trim().to_string();
                }
            }
            FormKind::Documento => {
                if self.form.d_nombre.trim().is_empty() {
                    self.avisar("Escribe el nombre del documento");
                    return;
                }
                if datos::parse_fecha(&self.form.d_vence).is_none() {
                    self.avisar("Escribe una fecha de vencimiento válida");
                    return;
                }
                let v = self.datos.indice();
                self.datos.docs.push(Documento {
                    vehiculo: v,
                    nombre: self.form.d_nombre.trim().to_string(),
                    vence: self.form.d_vence.trim().to_string(),
                });
            }
            FormKind::DocumentoEditar(i) => {
                if self.form.d_nombre.trim().is_empty() {
                    self.avisar("Escribe el nombre del documento");
                    return;
                }
                if datos::parse_fecha(&self.form.d_vence).is_none() {
                    self.avisar("Escribe una fecha de vencimiento válida");
                    return;
                }
                let v = self.datos.indice();
                let docs_indices: Vec<usize> = self.datos.docs.iter().enumerate()
                    .filter(|(_, d)| d.vehiculo == v)
                    .map(|(k, _)| k)
                    .collect();
                if let Some(&doc_idx) = docs_indices.get(i) {
                    if let Some(d) = self.datos.docs.get_mut(doc_idx) {
                        d.nombre = self.form.d_nombre.trim().to_string();
                        d.vence = self.form.d_vence.trim().to_string();
                    }
                }
            }
            FormKind::Perfil => {
                self.datos.perfil = Perfil {
                    nombre: self.form.p_nombre.trim().to_string(),
                    email: self.form.p_email.trim().to_string(),
                    ciudad: self.form.p_ciudad.trim().to_string(),
                    telefono: self.form.p_telefono.trim().to_string(),
                };
            }
            FormKind::Precio => {
                let p = a_dec(&self.form.precio_galon);
                if p > 0.0 {
                    self.datos.precio_galon = p;
                }
            }
        }
        self.guardar();
        self.avisar("Guardado");
        self.cerrar_form();
    }

    // ============================================================== toast
    fn toast_ui(&mut self, c: &Canvas) {
        let Some(t) = &self.toast else { return };
        let age = self.clock - t.at;
        if age > 2.6 {
            self.toast = None;
            return;
        }
        let in_a = ease_out((age / 0.22) as f32);
        let out_a = 1.0 - ease_out(((age - 2.2) / 0.4) as f32);
        let k = (in_a * out_a).clamp(0.0, 1.0);
        let y = 18.0 - (1.0 - in_a) * 14.0;
        let w = 230.0;
        // Centrado sobre el área de contenido (a la derecha del rail en tablet).
        let x0 = (self.lay.rail + self.lay.bw) * 0.5 - w * 0.5;
        let r = c.rect(x0, y, w, 38.0);
        c.p.rect_filled(r, Rounding::same(c.px(19.0)), c.ink(SURF3, 0.98 * k));
        c.p.rect_stroke(
            r,
            Rounding::same(c.px(19.0)),
            Stroke::new(1.0, c.ink(GREEN, 0.45 * k)),
        );
        c.p.circle_filled(
            c.pos(x0 + 22.0, y + 19.0),
            c.px(9.0),
            c.ink(GREEN, 0.2 * k),
        );
        icon(
            c,
            Ic::Check,
            c.pos(x0 + 22.0, y + 19.0),
            c.px(11.0),
            c.ink(GREEN, k),
        );
        txt(
            c,
            x0 + 38.0,
            y + 12.0,
            &t.texto,
            10.0,
            c.ink(TEXT, 0.95 * k),
            SEMI,
        );
    }

    // ============================================================== foto
    /// Miniatura: la foto se recorta para llenar el hueco.
    fn pintar_foto(&self, c: &Canvas, r: Rect, radius: f32) {
        match &self.foto {
            Some(tex) => {
                let (iw, ih) = (tex.size()[0] as f32, tex.size()[1] as f32);
                let fit = (r.width() / iw).min(r.height() / ih);
                let image_rect =
                    Rect::from_center_size(r.center(), Vec2::new(iw * fit, ih * fit));
                c.p.image(
                    tex.id(),
                    image_rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    c.ink(Color32::WHITE, 1.0),
                );
                c.p.rect_stroke(
                    r,
                    Rounding::same(c.px(radius)),
                    Stroke::new(c.px(radius) * 1.05, c.ink(SURF, 1.0)),
                );
            }
            None => {
                c.p.rect_filled(r, Rounding::same(c.px(radius)), c.ink(SURF, 1.0));
                silueta(c, r);
            }
        }
    }

    /// Foto grande sin marco: la moto flota sobre el fondo y los bordes de la
    /// foto se disuelven en el negro de la app (nada de recuadros ni bordes).
    fn pintar_hero(&self, c: &Canvas, r: Rect, _radius: f32) {
        let Some(tex) = &self.foto else {
            c.p.rect_filled(r, Rounding::ZERO, c.ink(SURF, 0.0));
            silueta(c, r);
            return;
        };
        let (iw, ih) = (tex.size()[0] as f32, tex.size()[1] as f32);
        let fit = (r.width() / iw).min(r.height() * 1.02 / ih);
        let full = Rect::from_center_size(r.center(), Vec2::new(iw * fit, ih * fit));

        // Composición deportiva: círculos, rejilla y resplandor rojo detrás de la moto.
        let cx = r.center().x;
        let cy = r.center().y + c.px(18.0);
        c.p.circle_filled(
            Pos2::new(cx, cy),
            c.px(124.0),
            c.ink(ACCENT_DEEP, 0.18),
        );
        c.p.circle_filled(
            Pos2::new(cx, cy),
            c.px(100.0),
            c.ink(ACCENT, 0.09),
        );
        c.p.circle_stroke(
            Pos2::new(cx, cy),
            c.px(112.0),
            Stroke::new(c.px(1.0), c.ink(ACCENT, 0.22)),
        );
        c.p.circle_stroke(
            Pos2::new(cx, cy),
            c.px(82.0),
            Stroke::new(c.px(1.0), c.ink(Color32::WHITE, 0.06)),
        );

        for i in 0..6 {
            let x = r.left() + c.px(28.0 + i as f32 * 62.0);
            c.p.line_segment(
                [Pos2::new(x, r.bottom()), Pos2::new(r.left() + c.px(170.0 + i as f32 * 24.0), r.top())],
                Stroke::new(c.px(0.8), c.ink(Color32::WHITE, 0.028)),
            );
        }
        for i in 0..4 {
            let y = r.top() + c.px(58.0 + i as f32 * 48.0);
            c.p.line_segment(
                [Pos2::new(r.left() + c.px(12.0), y), Pos2::new(r.right() - c.px(12.0), y)],
                Stroke::new(c.px(0.7), c.ink(Color32::WHITE, 0.024)),
            );
        }

        // Suelo iluminado bajo las llantas.
        let floor = Rect::from_center_size(
            Pos2::new(full.center().x, full.bottom() - c.px(12.0)),
            Vec2::new(full.width() * 0.78, c.px(22.0)),
        );
        c.p.rect_filled(
            floor,
            Rounding::same(c.px(11.0)),
            c.ink(Color32::WHITE, 0.035),
        );

        const GX: usize = 12;
        const GY: usize = 12;
        let mut mesh = egui::Mesh::with_texture(tex.id());
        for j in 0..=GY {
            for i in 0..=GX {
                let u = i as f32 / GX as f32;
                let v = j as f32 / GY as f32;
                let pos = Pos2::new(
                    full.left() + u * full.width(),
                    full.top() + v * full.height(),
                );
                mesh.vertices.push(egui::epaint::Vertex {
                    pos,
                    uv: Pos2::new(u, v),
                    color: Color32::WHITE,
                });
            }
        }
        for j in 0..GY {
            for i in 0..GX {
                let a = (j * (GX + 1) + i) as u32;
                let b = a + 1;
                let cc = a + (GX + 1) as u32;
                let d = cc + 1;
                mesh.indices.extend_from_slice(&[a, b, d, a, d, cc]);
            }
        }
        c.p.add(egui::Shape::mesh(mesh));
    }
}

// silueta de moto: líneas finas plateadas, elegante (marcador de posición)
fn silueta(c: &Canvas, r: Rect) {
    let p = |x: f32, y: f32| {
        Pos2::new(
            r.left() + x / 1000.0 * r.width(),
            r.top() + y / 560.0 * r.height(),
        )
    };
    let col = c.ink(MUTED, 0.55);
    let st = Stroke::new(1.6, col);
    let wheel = |center: Pos2, rad: f32| {
        c.p.circle_stroke(center, rad, Stroke::new(1.6, col));
        c.p.circle_stroke(center, rad * 0.42, Stroke::new(1.1, c.ink(MUTED, 0.35)));
        c.p.circle_filled(center, rad * 0.08, c.ink(ACCENT, 0.8));
    };
    let scale = r.height() / 560.0;
    wheel(p(252.0, 398.0), 96.0 * scale);
    wheel(p(762.0, 398.0), 92.0 * scale);
    c.p.line_segment([p(252.0, 398.0), p(566.0, 322.0)], Stroke::new(2.4, col));
    c.p.line_segment([p(806.0, 292.0), p(762.0, 398.0)], Stroke::new(2.4, col));

    c.p.add(egui::Shape::closed_line(
        vec![
            p(344.0, 212.0), p(392.0, 192.0), p(470.0, 202.0), p(560.0, 212.0),
            p(622.0, 202.0), p(700.0, 194.0), p(744.0, 210.0), p(736.0, 244.0),
            p(686.0, 252.0), p(636.0, 256.0), p(560.0, 262.0), p(470.0, 262.0),
            p(400.0, 248.0), p(352.0, 232.0),
        ],
        st,
    ));
    c.p.add(egui::Shape::closed_line(
        vec![
            p(744.0, 210.0), p(802.0, 202.0), p(860.0, 220.0), p(902.0, 260.0),
            p(910.0, 300.0), p(870.0, 320.0), p(828.0, 298.0), p(782.0, 282.0),
            p(748.0, 262.0), p(736.0, 244.0),
        ],
        st,
    ));
    c.p.add(egui::Shape::closed_line(
        vec![
            p(560.0, 262.0), p(636.0, 256.0), p(686.0, 252.0), p(736.0, 244.0),
            p(748.0, 262.0), p(782.0, 282.0), p(828.0, 298.0), p(818.0, 342.0),
            p(742.0, 356.0), p(660.0, 350.0), p(596.0, 332.0), p(556.0, 302.0),
        ],
        st,
    ));
    c.p.add(egui::Shape::line(
        vec![p(392.0, 200.0), p(470.0, 210.0), p(560.0, 220.0), p(622.0, 210.0)],
        Stroke::new(1.6, c.ink(ACCENT, 0.55)),
    ));
}

// --------------------------------------------------------------- arranque
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "reg".to_owned(),
        egui::FontData::from_static(include_bytes!("../Assets/Fonts/Barlow-Regular.ttf")),
    );
    fonts.font_data.insert(
        "med".to_owned(),
        egui::FontData::from_static(include_bytes!("../Assets/Fonts/Barlow-Medium.ttf")),
    );
    fonts.font_data.insert(
        "semi".to_owned(),
        egui::FontData::from_static(include_bytes!("../Assets/Fonts/Barlow-SemiBold.ttf")),
    );
    fonts.font_data.insert(
        "bold".to_owned(),
        egui::FontData::from_static(include_bytes!("../Assets/Fonts/Barlow-Bold.ttf")),
    );
    fonts.font_data.insert(
        "num".to_owned(),
        egui::FontData::from_static(include_bytes!("../Assets/Fonts/BarlowCondensed-SemiBold.ttf")),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "reg".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "reg".to_owned());
    for (name, list) in [
        ("reg", vec!["reg"]),
        ("med", vec!["med", "reg"]),
        ("semi", vec!["semi", "med", "reg"]),
        ("bold", vec!["bold", "semi", "reg"]),
        ("num", vec!["num", "semi", "reg"]),
    ] {
        fonts.families.insert(
            FontFamily::Name(name.into()),
            list.into_iter().map(|s| s.to_owned()).collect(),
        );
    }
    ctx.set_fonts(fonts);
}

fn load_photo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    // En el escritorio se puede reemplazar el archivo; en Android no hay una
    // carpeta de trabajo cómoda, así que la foto viaja dentro de la app.
    const FOTO_INCRUSTADA: &[u8] = include_bytes!("../Assets/Images/mi_moto.png");

    let candidates = [
        "Assets/Images/mi_moto.png",
        "Assets/Images/mi_moto.jpg",
        "../Assets/Images/mi_moto.png",
        "../Assets/Images/mi_moto.jpg",
        "app/Assets/Images/mi_moto.png",
        "app/Assets/Images/mi_moto.jpg",
    ];
    let bytes: std::borrow::Cow<'static, [u8]> = candidates
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .map(std::borrow::Cow::Owned)
        .unwrap_or(std::borrow::Cow::Borrowed(FOTO_INCRUSTADA));

    let img = image::load_from_memory(&bytes).ok()?;
    // las fotos de teléfono son enormes: se reducen para que quepan en
    // la tarjeta de vídeo y se vean nítidas igual
    let max_lado = 1024;
    let img = if img.width() > max_lado || img.height() > max_lado {
        let f = max_lado as f32 / img.width().max(img.height()) as f32;
        img.resize(
            (img.width() as f32 * f) as u32,
            (img.height() as f32 * f) as u32,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let color = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw());
    Some(ctx.load_texture("mi_moto", color, egui::TextureOptions::LINEAR))
}

// ------------------------------------------------------- arranque escritorio
/// Arranca la versión de escritorio. Acepta banderas de desarrollo:
/// `--screenshot <ruta>`, `--tab <inicio|garaje|manto|stats|perfil>`,
/// `--detail`, `--form <vehiculo|tanqueo|servicio|documento|perfil|precio>`,
/// `--foco <campo>`, `--demo` y `--vacio`.
pub fn desktop_main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut shot: Option<PathBuf> = None;
    let mut start_tab: Option<Tab> = None;
    let mut start_detail = false;
    let mut start_caja = false;
    let mut start_form: Option<FormKind> = None;
    let mut start_foco: Option<String> = None;
    let mut demo = false;
    let mut vacio = false;
    let mut size: Option<[f32; 2]> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" if i + 1 < args.len() => {
                shot = Some(PathBuf::from(&args[i + 1]));
                i += 2;
                continue;
            }
            // Tamaño de ventana en puntos, para probar el modo tablet en escritorio.
            "--size" if i + 1 < args.len() => {
                let partes: Vec<&str> = args[i + 1].split(['x', 'X']).collect();
                if partes.len() == 2 {
                    if let (Ok(w), Ok(h)) = (partes[0].parse(), partes[1].parse()) {
                        size = Some([w, h]);
                    }
                }
                i += 2;
                continue;
            }
            "--tab" if i + 1 < args.len() => {
                start_tab = Some(match args[i + 1].to_lowercase().as_str() {
                    "garaje" => Tab::Garaje,
                    "manto" | "mantenimiento" => Tab::Manto,
                    "stats" | "estadisticas" | "estadísticas" => Tab::Stats,
                    "perfil" => Tab::Perfil,
                    _ => Tab::Inicio,
                });
                i += 2;
                continue;
            }
            "--form" if i + 1 < args.len() => {
                start_form = Some(match args[i + 1].to_lowercase().as_str() {
                    "vehiculo" | "vehículo" => FormKind::VehiculoNuevo,
                    "tanqueo" => FormKind::Tanqueo,
                    "servicio" => FormKind::ServicioNuevo,
                    "documento" => FormKind::Documento,
                    "perfil" => FormKind::Perfil,
                    "precio" => FormKind::Precio,
                    _ => FormKind::Tanqueo,
                });
                i += 2;
                continue;
            }
            "--detail" => start_detail = true,
            "--caja" => start_caja = true,
            "--foco" if i + 1 < args.len() => {
                start_foco = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
            "--demo" => demo = true,
            "--vacio" | "--vacío" => vacio = true,
            _ => {}
        }
        i += 1;
    }
    let shot_mode = shot.is_some();

    let datos = if demo || (shot_mode && !vacio) {
        datos::sin_guardado(true);
        Datos::demo()
    } else if vacio {
        datos::sin_guardado(true);
        Datos::default()
    } else {
        datos::fijar_ruta(datos::ruta_escritorio());
        datos::cargar()
    };

    #[allow(unused_mut)]
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(size.unwrap_or([BASE_W, BASE_H]))
        .with_min_inner_size([360.0, 720.0])
        .with_title("Mi Garaje")
        .with_app_id("mi-garaje")
        .with_active(!shot_mode)
        .with_resizable(true);

    #[cfg(target_os = "windows")]
    if shot_mode {
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Mi Garaje",
        options,
        Box::new(move |cc| {
            let mut app = App::new(cc, shot.clone(), datos);
            if let Some(t) = start_tab {
                app.tab = t;
            }
            if start_detail {
                app.ir(Pantalla::Detalle(0));
            }
            if start_caja {
                app.abrir_caja();
            }
            if let Some(k) = start_form {
                app.abrir_form(k);
            }
            if let Some(f) = start_foco {
                if let Some(b) = app.form.buf_mut(&f) {
                    if b.is_empty() {
                        *b = "Yamaha".to_string();
                    }
                }
                app.foco = Some(f);
            }
            Ok(Box::new(app))
        }),
    )
}

// ---------------------------------------------------------- arranque Android
#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    // Guardamos la JVM y la Activity de inmediato: así, si algo falla más
    // adelante, podemos enseñar el error en la pantalla del teléfono.
    ime::fijar(app.vm_as_ptr(), app.activity_as_ptr());
    ime::marca("R1 android_main inicio");

    // Guardamos el último pánico para poder mostrarlo en pantalla.
    static ULTIMO_PANICO: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
    static HOOK: std::sync::Once = std::sync::Once::new();
    HOOK.call_once(|| {
        let anterior = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let loc = info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_default();
            let msg = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "panico sin mensaje".to_string());
            if let Ok(mut g) = ULTIMO_PANICO.lock() {
                *g = Some(format!("{msg}  (en {loc})"));
            }
            anterior(info);
        }));
    });

    // En Android el `AndroidApp` no es estado global: hay que entregárselo al
    // event loop de winit o `run_native` revienta al arrancar.
    let arranque = app.clone();

    let resultado = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        // Los datos viven en la carpeta privada de la app, así sobreviven a
        // actualizaciones y no requieren permisos de almacenamiento.
        ime::marca("R2 buscando carpeta de datos");
        if let Some(dir) = app.internal_data_path() {
            datos::fijar_ruta(dir.join("datos.json"));
        }
        let datos = datos::cargar();
        ime::marca("R3 datos cargados");

        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Mi Garaje"),
            event_loop_builder: Some(Box::new(move |builder| {
                builder.with_android_app(arranque);
            })),
            ..Default::default()
        };

        ime::marca("R4 run_native (crear ventana OpenGL)");
        let r = eframe::run_native(
            "Mi Garaje",
            options,
            Box::new(move |cc| {
                ime::marca("R5 creando App");
                let app = App::new(cc, None, datos);
                ime::marca("R6 App creada, primer fotograma pendiente");
                Ok(Box::new(app))
            }),
        );
        r.map_err(|e| format!("run_native fallo: {e}"))
    }));

    let aviso = match resultado {
        Ok(Ok(())) => "DIAGNOSTICO: el bucle de la app termino solo (sin error).".to_string(),
        Ok(Err(e)) => format!("DIAGNOSTICO: {e}"),
        Err(_) => {
            let msg = ULTIMO_PANICO
                .lock()
                .ok()
                .and_then(|g| g.clone())
                .unwrap_or_else(|| "panico sin mensaje".to_string());
            format!("DIAGNOSTICO panico:\n{msg}")
        }
    };
    eprintln!("{aviso}");

    // Lo enseñamos en la pantalla y dejamos la app abierta para que se lea.
    if ime::avisar(&aviso) {
        std::thread::sleep(std::time::Duration::from_secs(600));
    }
}
