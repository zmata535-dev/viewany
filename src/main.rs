use eframe::egui;
use image::{DynamicImage, GenericImageView};
use std::path::PathBuf;

fn generate_icon() -> Result<(), Box<dyn std::error::Error>> {
    let path = "viewany_icon.png";
    if std::path::Path::new(path).exists() {
        return Ok(());
    }
    let width = 256;
    let height = 256;
    let mut img = image::RgbaImage::new(width, height);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let is_border = x < 16 || x >= width - 16 || y < 16 || y >= height - 16;
        if is_border {
            let r = 212;
            let g = 175;
            let b = 55;
            *pixel = image::Rgba([r, g, b, 255]);
        } else {
            let dx = (x as f32 - 128.0) / 128.0;
            let dy = (y as f32 - 128.0) / 128.0;
            let dist = dx * dx + dy * dy;
            
            let is_head = dx * dx * 2.0 + (dy + 0.2) * (dy + 0.2) * 3.0 < 0.25;
            let is_body = dx * dx * 3.0 + (dy - 0.4) * (dy - 0.4) * 2.0 < 0.5;

            if is_head || is_body {
                let r = 160 + (dx * 30.0) as i32;
                let g = 120 + (dy * 30.0) as i32;
                let b = 80;
                *pixel = image::Rgba([r.clamp(0, 255) as u8, g.clamp(0, 255) as u8, b.clamp(0, 255) as u8, 255]);
            } else {
                let r = 40 + (dist * 20.0) as u8;
                let g = 45 + (dist * 20.0) as u8;
                let b = 50 + (dist * 20.0) as u8;
                *pixel = image::Rgba([r, g, b, 255]);
            }
        }
    }
    img.save(path)?;
    Ok(())
}

fn percent_decode(input: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = input.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next();
            let h2 = chars.next();
            if let (Some(c1), Some(c2)) = (h1, h2) {
                if let Ok(val) = u8::from_str_radix(std::str::from_utf8(&[c1, c2]).unwrap_or(""), 16) {
                    bytes.push(val);
                    continue;
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn parse_path_arg(arg: &str) -> PathBuf {
    let raw = if let Some(stripped) = arg.strip_prefix("file://") {
        stripped
    } else {
        arg
    };
    PathBuf::from(percent_decode(raw))
}

fn is_svg_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("svg") || e.eq_ignore_ascii_case("svgz"))
        .unwrap_or(false)
}

fn is_svg_bytes(bytes: &[u8]) -> bool {
    if let Ok(text) = std::str::from_utf8(bytes) {
        let trimmed = text.trim_start();
        trimmed.starts_with("<svg") || trimmed.starts_with("<?xml")
    } else {
        false
    }
}

fn load_svg_from_bytes(data: &[u8]) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let mut fontdb = resvg::usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    let opt = resvg::usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_data(data, &opt)?;
    let size = tree.size();
    let orig_w = size.width();
    let orig_h = size.height();

    // High quality scaling: if vector icon is small, render at least 1024px for crisp zoom
    let target_min_dim = 1024.0;
    let max_orig = orig_w.max(orig_h);
    let scale = if max_orig > 0.0 && max_orig < target_min_dim {
        target_min_dim / max_orig
    } else {
        1.0
    };
    let render_w = (orig_w * scale).round().max(1.0) as u32;
    let render_h = (orig_h * scale).round().max(1.0) as u32;

    let mut pixmap = resvg::tiny_skia::Pixmap::new(render_w, render_h)
        .ok_or("Failed to allocate pixmap for SVG rendering")?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Demultiply alpha from premultiplied tiny-skia pixels
    let premultiplied = pixmap.data();
    let mut straight_rgba = Vec::with_capacity(premultiplied.len());
    for chunk in premultiplied.chunks_exact(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];
        if a == 0 {
            straight_rgba.extend_from_slice(&[0, 0, 0, 0]);
        } else if a == 255 {
            straight_rgba.extend_from_slice(&[r, g, b, 255]);
        } else {
            let a_f = a as f32 / 255.0;
            let dr = ((r as f32 / a_f).round() as u8).min(255);
            let dg = ((g as f32 / a_f).round() as u8).min(255);
            let db = ((b as f32 / a_f).round() as u8).min(255);
            straight_rgba.extend_from_slice(&[dr, dg, db, a]);
        }
    }

    let rgba_img = image::RgbaImage::from_raw(render_w, render_h, straight_rgba)
        .ok_or("Failed to construct RGBA image from SVG pixmap")?;
    Ok(DynamicImage::ImageRgba8(rgba_img))
}

fn load_svg_from_path(path: &std::path::Path) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    if let Ok(data) = std::fs::read(path) {
        if let Ok(img) = load_svg_from_bytes(&data) {
            return Ok(img);
        }
    }

    // Fallback via rsvg-convert if available
    let temp_png = std::env::temp_dir().join("viewany_temp_svg_render.png");
    let status = std::process::Command::new("rsvg-convert")
        .arg("-o")
        .arg(&temp_png)
        .arg(path)
        .status();

    if status.is_ok() && status.unwrap().success() {
        if let Ok(img) = image::open(&temp_png) {
            let _ = std::fs::remove_file(&temp_png);
            return Ok(img);
        }
    }

    Err(format!("Could not render SVG: {}", path.display()).into())
}

fn save_svg_wrapper(path: &std::path::Path, img: &DynamicImage) -> Result<(), Box<dyn std::error::Error>> {
    let (w, h) = img.dimensions();
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    let svg_content = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
  <image xlink:href="data:image/png;base64,{b64}" width="{w}" height="{h}"/>
</svg>"#
    );
    std::fs::write(path, svg_content)?;
    Ok(())
}

struct ViewAnyApp {
    image_path: Option<PathBuf>,
    dynamic_image: Option<DynamicImage>,
    texture: Option<egui::TextureHandle>,
    show_menu: bool,
    status_message: String,
    zoom: f32,
    selecting_crop: bool,
    crop_start: Option<egui::Pos2>,
    crop_end: Option<egui::Pos2>,
    sibling_images: Vec<PathBuf>,
    sibling_index: usize,
    is_modified: bool,
    undo_stack: Vec<DynamicImage>,
    redo_stack: Vec<DynamicImage>,
    frame_count: u64,
    menu_open_frame: u64,
}

impl Default for ViewAnyApp {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ViewAnyApp {
    pub fn new(initial_path: Option<PathBuf>) -> Self {
        let _ = generate_icon();
        let mut app = Self {
            image_path: None,
            dynamic_image: None,
            texture: None,
            show_menu: false,
            status_message: "ViewAny: Right-click anywhere for menu. Scroll to zoom.".to_string(),
            zoom: 1.0,
            selecting_crop: false,
            crop_start: None,
            crop_end: None,
            sibling_images: Vec::new(),
            sibling_index: 0,
            is_modified: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            frame_count: 0,
            menu_open_frame: 0,
        };

        if let Some(path) = initial_path {
            if path.exists() {
                app.load_image(path);
                return app;
            }
        }

        // Try loading first image from Pictures directory as fallback
        let pictures_dir = std::env::var_os("XDG_PICTURES_DIR")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Pictures")));

        if let Some(dir) = pictures_dir {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ["jpg", "jpeg", "png", "webp", "bmp", "gif", "tiff", "tif", "ico", "svg", "svgz"]
                                .contains(&ext.to_lowercase().as_str())
                            {
                                app.load_image(path);
                                break;
                            }
                        }
                    }
                }
            }
        }

        app
    }
    fn load_image(&mut self, path: PathBuf) {
        let load_result = if is_svg_file(&path) {
            load_svg_from_path(&path)
        } else {
            image::open(&path).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        };

        match load_result {
            Ok(img) => {
                self.dynamic_image = Some(img);
                self.image_path = Some(path.clone());
                self.status_message = format!("Loaded: {}", path.display());
                self.texture = None;
                self.zoom = 1.0;
                self.selecting_crop = false;
                self.crop_start = None;
                self.crop_end = None;
                self.is_modified = false;
                self.undo_stack.clear();
                self.redo_stack.clear();
                self.update_siblings(&path);
            }
            Err(e) => {
                self.status_message = format!("Failed to load image: {}", e);
            }
        }
    }

    fn update_siblings(&mut self, current_path: &PathBuf) {
        self.sibling_images.clear();
        if let Some(parent) = current_path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                let mut paths: Vec<PathBuf> = entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_file()
                            && p.extension()
                                .and_then(|ext| ext.to_str())
                                .map(|ext| ["jpg", "jpeg", "png", "webp", "bmp", "gif", "tiff", "tif", "ico", "svg", "svgz"].contains(&ext.to_lowercase().as_str()))
                                .unwrap_or(false)
                    })
                    .collect();
                paths.sort();
                if let Some(idx) = paths.iter().position(|p| p == current_path) {
                    self.sibling_index = idx;
                }
                self.sibling_images = paths;
            }
        }
    }

    fn next_image(&mut self) {
        if !self.sibling_images.is_empty() {
            self.sibling_index = (self.sibling_index + 1) % self.sibling_images.len();
            let path = self.sibling_images[self.sibling_index].clone();
            self.load_image(path);
        } else {
            self.status_message = "No other images in folder.".to_string();
        }
    }

    fn prev_image(&mut self) {
        if !self.sibling_images.is_empty() {
            self.sibling_index = (self.sibling_index + self.sibling_images.len() - 1) % self.sibling_images.len();
            let path = self.sibling_images[self.sibling_index].clone();
            self.load_image(path);
        } else {
            self.status_message = "No other images in folder.".to_string();
        }
    }

    fn update_texture(&mut self, ctx: &egui::Context) {
        if let Some(img) = &self.dynamic_image {
            let size = [img.width() as usize, img.height() as usize];
            let image_buffer = img.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let egui_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
            self.texture = Some(ctx.load_texture("image-texture", egui_image, egui::TextureOptions::default()));
        }
    }

    fn push_history(&mut self) {
        if let Some(img) = &self.dynamic_image {
            self.undo_stack.push(img.clone());
            self.redo_stack.clear();
            self.is_modified = true;
        }
    }

    fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            if let Some(current) = self.dynamic_image.take() {
                self.redo_stack.push(current);
            }
            self.dynamic_image = Some(prev);
            self.texture = None;
            self.is_modified = !self.undo_stack.is_empty();
            self.status_message = "Undo performed".to_string();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            if let Some(current) = self.dynamic_image.take() {
                self.undo_stack.push(current);
            }
            self.dynamic_image = Some(next);
            self.texture = None;
            self.is_modified = true;
            self.status_message = "Redo performed".to_string();
        }
    }

    fn rotate_image(&mut self) {
        if let Some(img) = self.dynamic_image.clone() {
            self.push_history();
            self.dynamic_image = Some(img.rotate90());
            self.texture = None;
            self.status_message = "Rotated 90° clockwise".to_string();
        }
    }

    fn invert_colors(&mut self) {
        if let Some(mut img) = self.dynamic_image.clone() {
            self.push_history();
            img.invert();
            self.dynamic_image = Some(img);
            self.texture = None;
            self.status_message = "Inverted colors".to_string();
        }
    }

    fn save_image(&mut self) {
        if let (Some(path), Some(img)) = (&self.image_path, &self.dynamic_image) {
            let res = if is_svg_file(path) {
                save_svg_wrapper(path, img)
            } else {
                img.save(path).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
            };

            match res {
                Ok(_) => {
                    self.is_modified = false;
                    self.status_message = format!("Saved changes to {}", path.display());
                }
                Err(e) => {
                    self.status_message = format!("Failed to save: {}", e);
                }
            }
        } else {
            self.save_as();
        }
    }

    fn save_as(&mut self) {
        if let Some(img) = &self.dynamic_image {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "svg"])
                .add_filter("SVG Vector Graphics", &["svg"])
                .save_file()
            {
                let res = if is_svg_file(&path) {
                    save_svg_wrapper(&path, img)
                } else {
                    img.save(&path).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                };

                match res {
                    Ok(_) => {
                        self.image_path = Some(path.clone());
                        self.is_modified = false;
                        self.status_message = format!("Saved successfully to {}", path.display());
                        self.update_siblings(&path);
                    }
                    Err(e) => {
                        self.status_message = format!("Failed to save: {}", e);
                    }
                }
            }
        } else {
            self.status_message = "No image loaded to save!".to_string();
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("All Supported Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff", "tif", "ico", "svg", "svgz"])
            .add_filter("SVG Vector Graphics", &["svg", "svgz"])
            .add_filter("Raster Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tiff", "tif", "ico"])
            .pick_file()
        {
            self.load_image(path);
        }
    }

    fn copy_image_to_clipboard(&mut self) {
        if let Some(img) = &self.dynamic_image {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            let img_data = arboard::ImageData {
                width: width as usize,
                height: height as usize,
                bytes: std::borrow::Cow::Borrowed(rgba.as_raw()),
            };
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.set_image(img_data) {
                        Ok(_) => self.status_message = "Image copied to clipboard!".to_string(),
                        Err(e) => self.status_message = format!("Failed to copy image: {}", e),
                    }
                }
                Err(e) => self.status_message = format!("Clipboard error: {}", e),
            }
        } else {
            self.status_message = "No image loaded to copy!".to_string();
        }
    }

    fn copy_file_path(&mut self) {
        if let Some(path) = &self.image_path {
            let path_str = path.to_string_lossy().to_string();
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.set_text(path_str) {
                        Ok(_) => self.status_message = "File path copied to clipboard!".to_string(),
                        Err(e) => self.status_message = format!("Failed to copy path: {}", e),
                    }
                }
                Err(e) => self.status_message = format!("Clipboard error: {}", e),
            }
        } else {
            self.status_message = "No file path available!".to_string();
        }
    }

    fn go_to_location(&mut self) {
        if let Some(path) = &self.image_path {
            if let Some(parent) = path.parent() {
                let _ = open::that(parent);
                self.status_message = format!("Opened location: {}", parent.display());
            }
        } else {
            self.status_message = "No file loaded!".to_string();
        }
    }
}

fn menu_item(ui: &mut egui::Ui, text: &str, enabled: bool) -> bool {
    let btn = egui::Button::new(text).frame(false);
    let resp = ui.add_enabled(enabled, btn);
    resp.clicked()
}

impl eframe::App for ViewAnyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count += 1;

        // Handle Drag and Drop
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped_files.first() {
            if let Some(ref path) = file.path {
                self.load_image(path.clone());
            } else if let Some(ref bytes) = file.bytes {
                let mut loaded = false;
                if let Ok(text) = std::str::from_utf8(bytes) {
                    if let Some(first_line) = text.lines().next() {
                        let path = parse_path_arg(first_line.trim());
                        if path.exists() {
                            self.load_image(path);
                            loaded = true;
                        }
                    }
                }
                if !loaded {
                    let res = if is_svg_bytes(bytes) {
                        load_svg_from_bytes(bytes)
                    } else {
                        match image::load_from_memory(bytes) {
                            Ok(img) => Ok(img),
                            Err(_) => load_svg_from_bytes(bytes),
                        }
                    };

                    match res {
                        Ok(img) => {
                            self.dynamic_image = Some(img);
                            self.image_path = None;
                            self.status_message = "Loaded dropped image".to_string();
                            self.texture = None;
                            self.zoom = 1.0;
                            self.selecting_crop = false;
                            self.crop_start = None;
                            self.crop_end = None;
                            self.is_modified = false;
                            self.undo_stack.clear();
                            self.redo_stack.clear();
                        }
                        Err(e) => {
                            self.status_message = format!("Failed to load dropped image: {}", e);
                        }
                    }
                }
            }
        }

        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::M)) {
            self.show_menu = !self.show_menu;
            if self.show_menu {
                self.menu_open_frame = self.frame_count;
            }
        }

        // Global right-click anywhere to open menu
        if ctx.input(|i| i.pointer.secondary_clicked()) {
            self.show_menu = true;
            self.menu_open_frame = self.frame_count;
        }

        // ESC key handling
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.show_menu {
                self.show_menu = false;
            } else if self.selecting_crop {
                self.selecting_crop = false;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Keyboard shortcuts
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight) || i.key_pressed(egui::Key::PageDown)) {
            self.next_image();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp)) {
            self.prev_image();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S)) {
            if self.is_modified {
                self.save_image();
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::O)) {
            self.open_file_dialog();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) {
            if !self.undo_stack.is_empty() {
                self.undo();
            }
        }
        if ctx.input(|i| i.modifiers.ctrl && (i.key_pressed(egui::Key::Y) || (i.modifiers.shift && i.key_pressed(egui::Key::Z)))) {
            if !self.redo_stack.is_empty() {
                self.redo();
            }
        }

        // Scroll wheel zoom
        let scroll = ctx.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 && self.dynamic_image.is_some() {
            self.zoom = (self.zoom * (1.0 + scroll * 0.001)).clamp(0.1, 10.0);
        }

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.dynamic_image.is_some() {
                        if ui.button("▶ Next").clicked() {
                            self.next_image();
                        }
                        if ui.button("◀ Prev").clicked() {
                            self.prev_image();
                        }
                        ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let response = ui.interact(ui.max_rect(), ui.id(), egui::Sense::click_and_drag());

            if self.texture.is_none() && self.dynamic_image.is_some() {
                self.update_texture(ctx);
            }

            if let Some(texture) = &self.texture {
                let available_size = ui.available_size();
                let img_size = texture.size_vec2();
                
                let scale = (available_size.x / img_size.x).min(available_size.y / img_size.y).min(1.0) * self.zoom;
                let display_size = img_size * scale;

                let image_resp = ui.centered_and_justified(|ui| {
                    ui.image((texture.id(), display_size))
                });

                if self.selecting_crop {
                    if response.drag_started() {
                        self.crop_start = response.interact_pointer_pos();
                    }
                    if response.dragged() {
                        self.crop_end = ui.input(|i| i.pointer.interact_pos());
                    }
                    if response.drag_stopped() {
                        if let (Some(start), Some(end)) = (self.crop_start, self.crop_end) {
                            let rect = egui::Rect::from_two_pos(start, end);
                            if rect.width() > 10.0 && rect.height() > 10.0 {
                                let img_rect = image_resp.response.rect;
                                let (orig_w, orig_h) = self.dynamic_image.as_ref().unwrap().dimensions();
                                let scale_x = img_rect.width() / orig_w as f32;
                                let scale_y = img_rect.height() / orig_h as f32;

                                let x = ((rect.min.x - img_rect.min.x) / scale_x).clamp(0.0, orig_w as f32) as u32;
                                let y = ((rect.min.y - img_rect.min.y) / scale_y).clamp(0.0, orig_h as f32) as u32;
                                let w = (rect.width() / scale_x).clamp(1.0, orig_w as f32 - x as f32) as u32;
                                let h = (rect.height() / scale_y).clamp(1.0, orig_h as f32 - y as f32) as u32;

                                if w > 10 && h > 10 {
                                    self.push_history();
                                    if let Some(img) = self.dynamic_image.take() {
                                        self.dynamic_image = Some(img.crop_imm(x, y, w, h));
                                        self.texture = None;
                                        self.status_message = format!("Cropped selection: {}x{}", w, h);
                                    }
                                }
                            }
                        }
                        self.selecting_crop = false;
                        self.crop_start = None;
                        self.crop_end = None;
                    }

                    if let (Some(start), Some(current)) = (self.crop_start, self.crop_end) {
                        let rect = egui::Rect::from_two_pos(start, current);
                        ui.painter().rect_filled(
                            rect,
                            0.0,
                            egui::Color32::from_rgba_premultiplied(100, 150, 255, 60),
                        );
                    }
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.heading("ViewAny");
                        ui.label("No image loaded.");
                        if ui.button("Click here to Open Image (Ctrl+O)").clicked() {
                            self.open_file_dialog();
                        }
                        ui.label("Tip: Drag & drop image here or right-click anywhere for menu.");
                    });
                });
            }

            let is_hovering_file = ctx.input(|i| !i.raw.hovered_files.is_empty());
            if is_hovering_file {
                let painter = ui.painter();
                let rect = ui.max_rect();
                painter.rect_filled(
                    rect,
                    0.0,
                    egui::Color32::from_rgba_premultiplied(30, 100, 200, 45),
                );
                painter.rect_stroke(
                    rect.shrink(6.0),
                    6.0,
                    egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(100, 180, 255)),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Drop image here to open",
                    egui::FontId::proportional(22.0),
                    egui::Color32::WHITE,
                );
            }

            if self.show_menu {
                let window_response = egui::Window::new("ViewAny Menu")
                    .collapsible(false)
                    .resizable(false)
                    .title_bar(false)
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.vertical(|ui| {
                            ui.add_space(4.0);
                            ui.heading("ViewAny Menu");
                            ui.separator();

                            if menu_item(ui, "📁 Open Image... (Ctrl+O)", true) {
                                self.open_file_dialog();
                                self.show_menu = false;
                            }

                            if self.dynamic_image.is_some() {
                                if menu_item(ui, "💾 Save (Ctrl+S)", self.is_modified) {
                                    self.save_image();
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "💾 Save As...", true) {
                                    self.save_as();
                                    self.show_menu = false;
                                }
                                ui.separator();
                                if menu_item(ui, "↩ Undo (Ctrl+Z)", !self.undo_stack.is_empty()) {
                                    self.undo();
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "↪ Redo (Ctrl+Y)", !self.redo_stack.is_empty()) {
                                    self.redo();
                                    self.show_menu = false;
                                }
                                ui.separator();
                                if menu_item(ui, "🔄 Rotate 90°", true) {
                                    self.rotate_image();
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "🎨 Invert Colors", true) {
                                    self.invert_colors();
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "✂️ Selection Crop", true) {
                                    self.selecting_crop = true;
                                    self.status_message = "Drag on image to select crop area.".to_string();
                                    self.show_menu = false;
                                }
                                ui.separator();
                                if menu_item(ui, "🔍 Zoom In (+)", true) {
                                    self.zoom = (self.zoom * 1.25).min(10.0);
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "🔍 Zoom Out (-)", true) {
                                    self.zoom = (self.zoom / 1.25).max(0.1);
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "🔍 Reset Zoom (100%)", true) {
                                    self.zoom = 1.0;
                                    self.show_menu = false;
                                }
                                ui.separator();
                                if menu_item(ui, "▶ Next Image (Right Arrow)", !self.sibling_images.is_empty()) {
                                    self.next_image();
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "◀ Previous Image (Left Arrow)", !self.sibling_images.is_empty()) {
                                    self.prev_image();
                                    self.show_menu = false;
                                }
                                ui.separator();
                                if menu_item(ui, "📋 Copy Image to Clipboard", true) {
                                    self.copy_image_to_clipboard();
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "📋 Copy File Path", true) {
                                    self.copy_file_path();
                                    self.show_menu = false;
                                }
                                if menu_item(ui, "📂 Go to Location", true) {
                                    self.go_to_location();
                                    self.show_menu = false;
                                }
                            }
                            ui.separator();
                            if menu_item(ui, "🚪 Close App", true) {
                                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                self.show_menu = false;
                            }
                            ui.add_space(4.0);
                        });
                    });

                // Hide menu on click outside the pop-up window (only after the opening frame)
                if let Some(inner) = window_response {
                    if self.frame_count > self.menu_open_frame {
                        if ctx.input(|i| i.pointer.any_released()) {
                            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                                if !inner.response.rect.contains(pos) {
                                    self.show_menu = false;
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let initial_path = args.get(1).map(|arg| parse_path_arg(arg));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_title("ViewAny - Universal Image Viewer")
            .with_app_id("viewany")
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "ViewAny",
        options,
        Box::new(move |_cc| Ok(Box::new(ViewAnyApp::new(initial_path)))),
    )
}
