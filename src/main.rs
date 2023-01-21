use bsp::{Polygon};
use eframe::egui;
use rand::prelude::*;

mod bsp;

#[cfg(target_arch="wasm32")]
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
            Box::new(|cc| Box::new(MyEguiApp::new(cc)))
        )
        .await
        .expect("failed to start eframe");
    });
}

#[cfg(not(target_arch="wasm32"))]
fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "wrong!track!",
        native_options,
        Box::new(|cc| Box::new(MyEguiApp::new(cc))),
    );
}

struct MyEguiApp {
    // color
    bsp: bsp::Bsp<glam::Vec3>,

    normal_randomness: f32,
    color_randomness: f32,
    num_color_samples: usize,
}

impl MyEguiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.

        let bsp = bsp::Bsp::new(glam::Vec3::ONE);

        MyEguiApp { bsp,
            normal_randomness: 0.5,
            color_randomness: 0.5,
            num_color_samples: 3,
        }
    }
}
fn vec_to_color(v: glam::Vec3) -> egui::Color32 {
    let v = v * 256.0;
    let r = v.x as u8;
    let g = v.y as u8;
    let b = v.z as u8;

    egui::Color32::from_rgb(r, g, b)
}

impl eframe::App for MyEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut rng = thread_rng();
        egui::SidePanel::right("right").show(ctx, |ui| {
            ui.heading("wrong!track!");
            ui.monospace(format!("BSP nodes: {}", self.bsp.len()));

            if ui.button("CLEAR ALL").clicked() {
                self.bsp = bsp::Bsp::new(glam::Vec3::ONE);
            }

            let label = ui.label("Normal randomness").id;
            ui.add(egui::widgets::Slider::new(&mut self.normal_randomness, 0.0..=1.0))
                .labelled_by(label);

            let label = ui.label("Color randomness").id;
                ui.add(egui::widgets::Slider::new(&mut self.color_randomness, 0.0..=1.0))
                    .labelled_by(label);
            
            let label = ui.label("Num. color samples").id;
                ui.add(egui::widgets::DragValue::new(&mut self.num_color_samples).clamp_range(1..=8))
                    .labelled_by(label);

        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.separator();

            if ui.button("RND SPLIT [R]").clicked() || ui.input().key_pressed(egui::Key::R) {
                let rand_point = glam::Vec2::new(rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0));

                let rand_normal = glam::Vec2::new(rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0)).normalize();
                let rand_normal = (rand_point - glam::Vec2::new(0.5, 0.5)).lerp(rand_normal, self.normal_randomness);
                let rand_normal = rand_normal.normalize();
                
                let rand_color = glam::vec3(rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0));


                let mut nearby_color = glam::Vec3::ZERO;
                for _ in 0..self.num_color_samples {
                    let sample_point = (rand_point + 0.05 * glam::Vec2::new(rng.gen_range(0.0f32..1.0), rng.gen_range(0.0f32..1.0))).clamp(glam::Vec2::ZERO, glam::Vec2::ONE);
                    let sample_color = self.bsp.get_at_point(sample_point);
                    nearby_color += *sample_color * (1.0 / self.num_color_samples as f32);
                }
            
                let color = nearby_color.lerp(rand_color, self.color_randomness);


                self.bsp.split_at_point(rand_point, rand_normal, color);
            }
            ui.separator();
            let sense = egui::Sense::focusable_noninteractive();
            let (response, painter) = ui.allocate_painter(egui::Vec2::new(512.0, 512.0), sense);

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
