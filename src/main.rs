use base64::Engine;
use bsp::Polygon;
use eframe::egui;
use palette::{IntoColor, Mix, Oklab, Srgb};
use rand::prelude::*;

mod bsp;

#[cfg(target_arch = "wasm32")]
fn main() {
    use wasm_bindgen::prelude::*;

    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("canvas").unwrap();
    val.set_id("canvas");
    body.append_child(&val);

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "canvas", // hardcode it
            web_options,
            Box::new(|cc| Box::new(MyEguiApp::new(cc))),
        )
        .await
        .expect("failed to start eframe");
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "wrong!track!",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    );
}

#[derive(Debug, PartialEq, Eq)]
enum Tool {
    Split,
    Unsplit,
    Paint
}
struct MyEguiApp {
    // color
    bsp: bsp::Bsp<Oklab>,

    normal_randomness: f32,
    color_randomness: f32,
    num_color_samples: usize,

    drag_start_pos: egui::Pos2,

    override_color_enabled: bool,
    override_color: egui::epaint::Hsva,

    tool: Tool,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        let bsp = bsp::Bsp::new(Oklab::new(1.0, 0.0, 0.0));

        MyEguiApp {
            bsp,

            normal_randomness: 0.5,
            color_randomness: 0.5,
            num_color_samples: 3,
            override_color_enabled: false,
            override_color: egui::epaint::Hsva::default(),

            drag_start_pos: egui::Pos2::ZERO,

            tool: Tool::Split,
        }
    }
}
fn vec_to_color(v: Oklab) -> egui::Color32 {
    let v: Srgb = v.into_color();

    egui::Color32::from_rgb(
        (v.red * 256.0) as u8,
        (v.green * 256.0) as u8,
        (v.blue * 256.0) as u8,
    )
}

impl MyEguiApp {
    fn random_point_in_disk(&self, radius: f32) -> glam::Vec2 {
        let mut rng = thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::PI * 2.0);
        let distance = rng.gen_range(0.0..radius);
        let (x, y) = angle.sin_cos();
        glam::vec2(x, y) * distance
    }

    fn random_color(&self, point: glam::Vec2) -> Oklab {
        let mut rng = thread_rng();
        if self.override_color_enabled {
            let oc = self.override_color.to_srgb().map(|x| x as f32 / 256.0);
            Srgb::from_components((oc[0], oc[1], oc[2])).into_color()
        } else {
            let random_color = Srgb::from_components((
                rng.gen_range(0.0f32..1.0),
                rng.gen_range(0.0f32..1.0),
                rng.gen_range(0.0f32..1.0),
            ))
            .into_color();
            let mut sampled_color = Oklab::default();
            for _ in 0..self.num_color_samples {
                let perturb = self.random_point_in_disk(0.05);
                let sample_point = (point + perturb).clamp(glam::Vec2::ZERO, glam::Vec2::ONE);
                let sample = self.bsp.get_at_point(sample_point);
                sampled_color += *sample;
            }
            sampled_color *= 1.0 / (self.num_color_samples as f32);

            sampled_color.mix(&random_color, self.color_randomness)
        }
    }
    fn random_normal(&self, point: glam::Vec2) -> glam::Vec2 {
        let mut rng = thread_rng();
        let angle = rng.gen_range(0.0..std::f32::consts::PI * 2.0);
        let (x, y) = angle.sin_cos();
        let rand_normal = glam::vec2(x, y);

        let rand_normal =
            (point - glam::Vec2::new(0.5, 0.5)).lerp(rand_normal, self.normal_randomness);
        rand_normal.normalize()
    }
}
impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut rng = thread_rng();

        egui::SidePanel::right("right")
            .max_width(512.0)
            .show(ctx, |ui| {
                ui.heading("wrong!track!");
                ui.hyperlink_to(
                    "source on GitHub",
                    "https://github.com/XMPPwocky/wrongtrack",
                );

                ui.monospace(format!("BSP nodes: {}", self.bsp.len()));

                if ui.button("CLEAR ALL").clicked() {
                    self.bsp = bsp::Bsp::new(Oklab::new(1.0, 0.0, 0.0));
                }

                if ui.button("Export SVG").clicked() {
                    let url = to_data_url(save_svg(&self.bsp));
                    open_url_new_tab(ui.ctx(), &url);
                }

                let label = ui.label("Normal randomness").id;
                ui.add(egui::widgets::Slider::new(
                    &mut self.normal_randomness,
                    0.0..=1.0,
                ))
                .labelled_by(label);

                ui.heading("Color");

                let label = ui.label("Color randomness").id;
                ui.add(egui::widgets::Slider::new(
                    &mut self.color_randomness,
                    0.0..=1.0,
                ))
                .labelled_by(label);

                let label = ui.label("Num. color samples").id;
                ui.add(
                    egui::widgets::DragValue::new(&mut self.num_color_samples).clamp_range(1..=64),
                )
                .labelled_by(label);

                ui.group(|ui| {
                    ui.checkbox(&mut self.override_color_enabled, "Override color?");
                    ui.add_enabled_ui(self.override_color_enabled, |ui| {
                        egui::widgets::color_picker::color_picker_hsva_2d(
                            ui,
                            &mut self.override_color,
                            egui::widgets::color_picker::Alpha::Opaque,
                        )
                    });
                });
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("RND SPLIT [R]").clicked() || ui.input().key_pressed(egui::Key::R) {
                    let rand_point =
                        glam::Vec2::new(rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0));

                    let rand_normal = self.random_normal(rand_point);

                    let rand_color = self.random_color(rand_point);

                    self.bsp.split_at_point(rand_point, rand_normal, rand_color);
                }
                if ui.button("SPLIT X100").clicked() {
                    for _ in 0..100 {
                        let rand_point =
                            glam::Vec2::new(rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0));

                        let rand_normal = self.random_normal(rand_point);

                        let rand_color = self.random_color(rand_point);

                        self.bsp.split_at_point(rand_point, rand_normal, rand_color);
                    }
                }
                ui.radio_value(&mut self.tool, Tool::Split, "Split");
                ui.radio_value(&mut self.tool, Tool::Unsplit, "Unsplit");
                ui.radio_value(&mut self.tool, Tool::Paint, "Paint");
            });

            ui.separator();
            let sense = egui::Sense::click_and_drag();
            let (response, painter) = ui.allocate_painter(egui::Vec2::new(512.0, 512.0), sense);
            if self.tool == Tool::Split {
                if response.hovered() {
                    ui.ctx().output().cursor_icon = egui::CursorIcon::Crosshair;
                }
                if response.clicked() && !response.drag_released() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let response_size = response.rect.size();

                        let rel_pos = (pos - response.rect.min) / response_size;

                        let rand_normal =
                            glam::Vec2::new(rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0))
                                .normalize();
                        let rand_color =
                            self.random_color(<[f32; 2] as From<_>>::from(rel_pos).into());

                        self.bsp.split_at_point(
                            <[f32; 2] as From<_>>::from(rel_pos).into(),
                            rand_normal,
                            rand_color,
                        );
                    }
                }
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.drag_start_pos = pos;
                    }
                }
                if response.drag_released() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let response_size = response.rect.size();

                        let middle_pos = (pos + self.drag_start_pos.to_vec2()).to_vec2() * 0.5;
                        let rel_pos = (middle_pos - response.rect.min.to_vec2()) / response_size;
                        let rel_pos = <[f32; 2] as From<_>>::from(rel_pos).into();

                        let rand_color = self.random_color(rel_pos);

                        let drag_normal = (pos - self.drag_start_pos).normalized();
                        let drag_normal = if drag_normal.length_sq() < 1e-6 {
                            self.random_normal(rel_pos)
                        } else {
                            <[f32; 2] as From<_>>::from(drag_normal).into()
                        };
                        self.bsp.split_at_point(rel_pos, drag_normal, rand_color);
                    }
                }
            } else if self.tool == Tool::Paint {
                if response.hovered() {
                    ui.ctx().output().cursor_icon = egui::CursorIcon::Crosshair;
                }
                if response.is_pointer_button_down_on() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let response_size = response.rect.size();

                        let rel_pos = (pos - response.rect.min) / response_size;
                        let rel_pos = <egui::Vec2 as Into<[f32; 2]>>::into(rel_pos).into();

                        *self.bsp.get_at_point_mut(rel_pos) = self.random_color(rel_pos);
                    }
                }
            } else if self.tool == Tool::Unsplit {
                if response.hovered() {
                    ui.ctx().output().cursor_icon = egui::CursorIcon::Crosshair;
                }
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let response_size = response.rect.size();

                        let rel_pos = (pos - response.rect.min) / response_size;
                        let rel_pos = <egui::Vec2 as Into<[f32; 2]>>::into(rel_pos).into();

                        self.bsp.unsplit_at_point(rel_pos);
                    }
                }               
            }

            let outer_poly =
                bsp::Polygon::new_rect(glam::Vec2::new(0.0, 0.0), glam::Vec2::new(1.0, 1.0));

            self.bsp
                .visit_leaf_polygons(self.bsp.root_key(), outer_poly, &mut |leaf, poly| {
                    let fill_color = leaf;

                    let shapy = egui::Shape::Path(egui::epaint::PathShape {
                        closed: true,
                        fill: vec_to_color(fill_color.0),
                        stroke: egui::Stroke::new(0.0, egui::Color32::TRANSPARENT),
                        points: poly_to_egui_points(&poly, response.rect),
                    });
                    painter.add(shapy);
                });
        });
    }
}

fn poly_to_egui_points(poly: &Polygon, out_rect: egui::Rect) -> Vec<egui::Pos2> {
    let x_range = out_rect.x_range();
    let x_dist = x_range.end() - x_range.start();
    let y_range = out_rect.y_range();
    let y_dist = y_range.end() - y_range.start();

    poly.vertices
        .iter()
        .map(|&vert| {
            let x = x_range.start() + (vert.x * x_dist);
            let y = y_range.start() + (vert.y * y_dist);
            egui::Pos2::new(x, y)
        })
        .collect()
}

fn save_svg(bsp: &bsp::Bsp<Oklab>) -> Vec<u8> {
    use svg::node::element::path::Data;
    use svg::node::element::Path;
    use svg::Document;

    let mut document = Some(Document::new().set("viewBox", (0.0, 0.0, 1.0, 1.0)));

    bsp.visit_leaf_polygons(
        bsp.root_key(),
        bsp::Polygon::new_rect(glam::Vec2::ZERO, glam::Vec2::ONE),
        &mut |leaf, poly| {
            let color: Srgb = leaf.0.into_color();
            let color = format!(
                "rgb({}, {}, {})",
                (color.red * 256.0) as u8,
                (color.green * 256.0) as u8,
                (color.blue * 256.0) as u8
            );

            let mut data = Data::new();
            data = data.move_to((poly.vertices[0].x, poly.vertices[0].y));
            for vert in &poly.vertices[1..] {
                data = data.line_to((vert.x, vert.y));
            }
            data = data.close();

            let path = Path::new()
                .set("fill", color)
                .set("stroke", "none")
                .set("stroke-width", 0)
                .set("d", data);

            document = Some(document.take().unwrap().add(path));
        },
    );

    let mut w = Vec::new();
    svg::write(&mut w, &document.unwrap()).unwrap();
    w
}

fn to_data_url(data: Vec<u8>) -> String {
    let data = base64::engine::general_purpose::STANDARD_NO_PAD.encode(data);
    let url = format!("data:image/svg+xml;base64,{}", data);
    url
}

#[cfg(not(target_arch = "wasm32"))]
fn open_url_new_tab(ctx: &egui::Context, url: &str) {
    ctx.output().open_url(url);
}

#[cfg(target_arch = "wasm32")]
fn open_url_new_tab(_ctx: &egui::Context, url: &str) {
    wasm::save_data_url(url.to_owned());
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(module = "/save.js")]
    extern "C" {
        pub fn save_data_url(x: String);
    }
}
