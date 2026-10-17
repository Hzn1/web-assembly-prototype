use macroquad::math::{Vec2, Vec3};
use macroquad::prelude::*;
use macroquad::rand::{gen_range, srand};

// --- CONFIGURAÇÃO DA JANELA ---
fn window_conf() -> Conf {
    Conf {
        window_title: "Voxel Island".to_string(),
        high_dpi: true,
        sample_count: 1,
        window_resizable: true,
        ..Default::default()
    }
}

// --- CONSTANTES E TIPOS ---
const WX: usize = 48;
const WY: usize = 24;
const WZ: usize = 48;
const AIR: u8 = 0;
const GRASS: u8 = 1;
const DIRT: u8 = 2;
const STONE: u8 = 3;
const SAND: u8 = 4;
const WOOD: u8 = 5;
const LEAVES: u8 = 6;
const WATER: u8 = 7;
const PLANKS: u8 = 8;

const SKY_COLOR: Color = Color::new(0.4, 0.7, 1.0, 1.0);
const FOG_COLOR: Color = SKY_COLOR;

// --- MATEMÁTICA E UTILITÁRIOS ---
fn hash2(x: i32, y: i32) -> f32 {
    let mut h = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    ((h ^ (h >> 16)) as f32) / 2147483648.0
}

fn noise2(x: f32, y: f32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let xf = x - x.floor();
    let yf = y - y.floor();
    let u = xf * xf * (3.0 - 2.0 * xf);
    let v = yf * yf * (3.0 - 2.0 * yf);
    let a = hash2(xi, yi);
    let b = hash2(xi + 1, yi);
    let c = hash2(xi, yi + 1);
    let d = hash2(xi + 1, yi + 1);
    a + u * (b - a) + v * (c - a) + u * v * (a - b - c + d)
}

fn fbm(x: f32, y: f32) -> f32 {
    let mut val = 0.0;
    let mut amp = 0.5;
    let mut freq = 1.0;
    for _ in 0..4 {
        val += amp * noise2(x * freq, y * freq);
        amp *= 0.5;
        freq *= 2.0;
    }
    val
}

fn lerp_color(c1: Color, c2: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        c1.r + (c2.r - c1.r) * t,
        c1.g + (c2.g - c1.g) * t,
        c1.b + (c2.b - c1.b) * t,
        c1.a + (c2.a - c1.a) * t,
    )
}

fn block_color(btype: u8, face: usize, x: i32, y: i32, z: i32) -> Color {
    let hx = x.wrapping_mul(13).wrapping_add(y.wrapping_mul(7));
    let variation = hash2(hx, z.wrapping_mul(19)) * 0.1 - 0.05;
    let mut c = match btype {
        GRASS => match face {
            0 => Color::new(0.3, 0.8, 0.3, 1.0),
            1 => Color::new(0.5, 0.35, 0.2, 1.0),
            _ => Color::new(0.4, 0.25, 0.15, 1.0),
        },
        DIRT => Color::new(0.5, 0.35, 0.2, 1.0),
        STONE => Color::new(0.5, 0.5, 0.5, 1.0),
        SAND => Color::new(0.9, 0.85, 0.6, 1.0),
        WOOD => match face {
            0 | 1 => Color::new(0.6, 0.5, 0.3, 1.0),
            _ => Color::new(0.4, 0.25, 0.1, 1.0),
        },
        LEAVES => Color::new(0.2, 0.6, 0.2, 1.0),
        WATER => {
            let anim = (get_time() * 2.0).sin() as f32 * 0.1;
            Color::new(0.2, 0.4 + anim, 0.8, 0.7)
        }
        PLANKS => Color::new(0.7, 0.5, 0.3, 1.0),
        _ => Color::new(1.0, 0.0, 1.0, 1.0),
    };

    let shade = match face {
        0 => 1.0,
        1 => 0.6,
        2 | 3 => 0.8,
        4 | 5 => 0.7,
        _ => 1.0,
    };

    c.r = (c.r * shade + variation).clamp(0.0, 1.0);
    c.g = (c.g * shade + variation).clamp(0.0, 1.0);
    c.b = (c.b * shade + variation).clamp(0.0, 1.0);
    c
}

// --- MUNDO ---
struct World {
    blocks: Vec<u8>,
}

impl World {
    fn new() -> Self {
        let mut w = World {
            blocks: vec![AIR; WX * WY * WZ],
        };
        w.generate();
        w
    }

    fn idx(x: i32, y: i32, z: i32) -> usize {
        (x as usize) + (y as usize) * WX + (z as usize) * WX * WY
    }

    fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if x < 0 || x >= WX as i32 || y < 0 || y >= WY as i32 || z < 0 || z >= WZ as i32 {
            return AIR;
        }
        self.blocks[Self::idx(x, y, z)]
    }

    fn set(&mut self, x: i32, y: i32, z: i32, val: u8) {
        if x >= 0 && x < WX as i32 && y >= 0 && y < WY as i32 && z >= 0 && z < WZ as i32 {
            self.blocks[Self::idx(x, y, z)] = val;
        }
    }

    fn generate(&mut self) {
        let cx = WX as f32 / 2.0;
        let cz = WZ as f32 / 2.0;
        let max_r = 20.0;

        for x in 0..WX {
            for z in 0..WZ {
                let dx = x as f32 - cx;
                let dz = z as f32 - cz;
                let dist = (dx * dx + dz * dz).sqrt();
                let mask = (1.0 - (dist / max_r).clamp(0.0, 1.0)).powf(1.5);

                let h = (fbm(x as f32 * 0.05, z as f32 * 0.05) * 12.0 * mask + 8.0) as i32;
                let water_level = 10;

                for y in 0..WY {
                    let y_i32 = y as i32;
                    if y_i32 > h && y_i32 <= water_level {
                        self.set(x as i32, y_i32, z as i32, WATER);
                    } else if y_i32 <= h {
                        if y_i32 == h {
                            if h <= water_level + 1 {
                                self.set(x as i32, y_i32, z as i32, SAND);
                            } else {
                                self.set(x as i32, y_i32, z as i32, GRASS);
                            }
                        } else if y_i32 > h - 3 {
                            self.set(x as i32, y_i32, z as i32, DIRT);
                        } else {
                            self.set(x as i32, y_i32, z as i32, STONE);
                        }
                    }
                }
            }
        }

        let mut tree_count = 0;
        for _ in 0..50 {
            let tx = gen_range(4i32, (WX - 4) as i32);
            let tz = gen_range(4i32, (WZ - 4) as i32);
            let mut ty = WY as i32 - 1;
            while ty > 0 && self.get(tx, ty, tz) == AIR {
                ty -= 1;
            }

            if self.get(tx, ty, tz) == GRASS && ty > 11 {
                let mut too_close = false;
                for i in -3..=3 {
                    for j in -3..=3 {
                        if self.get(tx + i, ty + 1, tz + j) == LEAVES {
                            too_close = true;
                            break;
                        }
                    }
                    if too_close {
                        break;
                    }
                }

                if !too_close {
                    let tree_h = gen_range(3i32, 6i32);
                    for i in 1..=tree_h {
                        self.set(tx, ty + i, tz, WOOD);
                    }
                    for lx in -2..=2 {
                        for ly in -1..=2 {
                            for lz in -2..=2 {
                                if lx * lx + ly * ly + lz * lz <= 5 {
                                    let bx = tx + lx;
                                    let by = ty + tree_h + ly;
                                    let bz = tz + lz;
                                    if self.get(bx, by, bz) == AIR {
                                        self.set(bx, by, bz, LEAVES);
                                    }
                                }
                            }
                        }
                    }
                    tree_count += 1;
                    if tree_count >= 12 {
                        break;
                    }
                }
            }
        }
    }
}

// --- JOGADOR E FÍSICA ---
struct Player {
    pos: Vec3,
    vel: Vec3,
    yaw: f32,
    pitch: f32,
    on_ground: bool,
    in_water: bool,
    selected_block: u8,
}

impl Player {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: Vec3::new(x, y, z),
            vel: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
            in_water: false,
            selected_block: PLANKS,
        }
    }

    fn eye_pos(&self) -> Vec3 {
        self.pos + Vec3::new(0.0, 1.6, 0.0)
    }

    fn forward(&self) -> Vec3 {
        let cp = self.pitch.cos();
        let sp = self.pitch.sin();
        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        Vec3::new(-sy * cp, sp, -cy * cp)
    }
}

fn collides(world: &World, pos: Vec3, w: f32, h: f32) -> bool {
    let min_x = (pos.x - w / 2.0).floor() as i32;
    let max_x = (pos.x + w / 2.0).floor() as i32;
    let min_y = pos.y.floor() as i32;
    let max_y = (pos.y + h).floor() as i32;
    let min_z = (pos.z - w / 2.0).floor() as i32;
    let max_z = (pos.z + w / 2.0).floor() as i32;

    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let b = world.get(x, y, z);
                if b != AIR && b != WATER {
                    return true;
                }
            }
        }
    }
    false
}

fn update_physics(player: &mut Player, world: &World, move_dir: Vec2, jump: bool, dt: f32) {
    let speed = if player.in_water { 3.0 } else { 5.0 };
    let fwd = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let rgt = Vec3::new(player.yaw.cos(), 0.0, -player.yaw.sin());

    let wish_vel = fwd * move_dir.y * speed + rgt * move_dir.x * speed;

    if player.in_water {
        player.vel.x = wish_vel.x;
        player.vel.z = wish_vel.z;
        player.vel.y *= 0.8;
        if jump {
            player.vel.y = 2.0;
        } else {
            player.vel.y -= 2.0 * dt;
        }
    } else {
        player.vel.x = wish_vel.x;
        player.vel.z = wish_vel.z;
        if jump && player.on_ground {
            player.vel.y = 7.0;
            player.on_ground = false;
        }
        player.vel.y -= 20.0 * dt;
    }

    let w = 0.6;
    let h = 1.8;

    player.pos.x += player.vel.x * dt;
    if collides(world, player.pos, w, h) {
        player.pos.x -= player.vel.x * dt;
        player.vel.x = 0.0;
    }

    player.pos.z += player.vel.z * dt;
    if collides(world, player.pos, w, h) {
        player.pos.z -= player.vel.z * dt;
        player.vel.z = 0.0;
    }

    player.pos.y += player.vel.y * dt;
    player.on_ground = false;
    if collides(world, player.pos, w, h) {
        player.pos.y -= player.vel.y * dt;
        if player.vel.y < 0.0 {
            player.on_ground = true;
        }
        player.vel.y = 0.0;
    }

    let head_b = world.get(
        player.pos.x as i32,
        (player.pos.y + 1.6) as i32,
        player.pos.z as i32,
    );
    player.in_water = head_b == WATER;

    if player.pos.y < -10.0 {
        player.pos = Vec3::new(WX as f32 / 2.0, 20.0, WZ as f32 / 2.0);
        player.vel = Vec3::ZERO;
    }
}

// --- RAYCAST (DDA) ---
fn raycast(world: &World, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<(Vec3, Vec3)> {
    let mut x = origin.x.floor() as i32;
    let mut y = origin.y.floor() as i32;
    let mut z = origin.z.floor() as i32;

    let step_x = if dir.x > 0.0 { 1 } else { -1 };
    let step_y = if dir.y > 0.0 { 1 } else { -1 };
    let step_z = if dir.z > 0.0 { 1 } else { -1 };

    let t_delta_x = if dir.x != 0.0 {
        (1.0 / dir.x).abs()
    } else {
        f32::MAX
    };
    let t_delta_y = if dir.y != 0.0 {
        (1.0 / dir.y).abs()
    } else {
        f32::MAX
    };
    let t_delta_z = if dir.z != 0.0 {
        (1.0 / dir.z).abs()
    } else {
        f32::MAX
    };

    let mut t_max_x = if dir.x > 0.0 {
        (x as f32 + 1.0 - origin.x) / dir.x
    } else if dir.x < 0.0 {
        (origin.x - x as f32) / -dir.x
    } else {
        f32::MAX
    };
    let mut t_max_y = if dir.y > 0.0 {
        (y as f32 + 1.0 - origin.y) / dir.y
    } else if dir.y < 0.0 {
        (origin.y - y as f32) / -dir.y
    } else {
        f32::MAX
    };
    let mut t_max_z = if dir.z > 0.0 {
        (z as f32 + 1.0 - origin.z) / dir.z
    } else if dir.z < 0.0 {
        (origin.z - z as f32) / -dir.z
    } else {
        f32::MAX
    };

    let mut normal = Vec3::ZERO;
    #[allow(unused_assignments)]
    let mut dist = 0.0;

    for _ in 0..(max_dist * 2.0) as i32 {
        let b = world.get(x, y, z);
        if b != AIR && b != WATER {
            return Some((Vec3::new(x as f32, y as f32, z as f32), normal));
        }

        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                x += step_x;
                dist = t_max_x;
                t_max_x += t_delta_x;
                normal = Vec3::new(-step_x as f32, 0.0, 0.0);
            } else {
                z += step_z;
                dist = t_max_z;
                t_max_z += t_delta_z;
                normal = Vec3::new(0.0, 0.0, -step_z as f32);
            }
        } else {
            if t_max_y < t_max_z {
                y += step_y;
                dist = t_max_y;
                t_max_y += t_delta_y;
                normal = Vec3::new(0.0, -step_y as f32, 0.0);
            } else {
                z += step_z;
                dist = t_max_z;
                t_max_z += t_delta_z;
                normal = Vec3::new(0.0, 0.0, -step_z as f32);
            }
        }
        if dist > max_dist {
            break;
        }
    }
    None
}

// --- RENDERER 3D SOFTWARE ---
struct Face {
    v: [Vec2; 4],
    color: Color,
    dist_sq: f32,
    lod: bool,
}

fn project(v: Vec3, cam: &Player, sw: f32, sh: f32) -> Option<Vec2> {
    let mut p = v - cam.eye_pos();

    let cy = cam.yaw.cos();
    let sy = cam.yaw.sin();
    let x = p.x * cy - p.z * sy;
    let z = p.x * sy + p.z * cy;
    p.x = x;
    p.z = z;

    let cp = cam.pitch.cos();
    let sp = cam.pitch.sin();
    let y = p.y * cp - p.z * sp;
    let z2 = p.y * sp + p.z * cp;
    p.y = y;
    p.z = z2;

    if p.z < 0.2 {
        return None;
    }

    let focal = sh / (2.0 * (45.0_f32.to_radians() / 2.0).tan());
    let sx = (p.x / p.z) * focal + sw / 2.0;
    let sy = -(p.y / p.z) * focal + sh / 2.0;
    Some(Vec2::new(sx, sy))
}

fn draw_face(face: &Face) {
    let mut v = face.v;
    let cx = (v[0].x + v[1].x + v[2].x + v[3].x) / 4.0;
    let cy = (v[0].y + v[1].y + v[2].y + v[3].y) / 4.0;
    for i in 0..4 {
        let dx = v[i].x - cx;
        let dy = v[i].y - cy;
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        v[i].x += (dx / len) * 0.5;
        v[i].y += (dy / len) * 0.5;
    }

    draw_triangle(v[0], v[1], v[2], face.color);
    draw_triangle(v[0], v[2], v[3], face.color);

    if face.lod {
        let _light = Color::new(
            face.color.r * 1.1,
            face.color.g * 1.1,
            face.color.b * 1.1,
            face.color.a,
        );
        let dark = Color::new(
            face.color.r * 0.7,
            face.color.g * 0.7,
            face.color.b * 0.7,
            face.color.a,
        );

        let dx = (v[1].x - v[0].x) * 0.2;
        let dy = (v[1].y - v[0].y) * 0.2;
        let dx2 = (v[3].x - v[0].x) * 0.2;
        let dy2 = (v[3].y - v[0].y) * 0.2;

        let p1 = v[0] + Vec2::new(dx + dx2, dy + dy2);
        let p2 = p1 + Vec2::new(dx, dy);
        let p3 = p1 + Vec2::new(dx2, dy2);
        draw_triangle(p1, p2, p3, dark);
    }
}

// --- LOOP PRINCIPAL ---
#[macroquad::main(window_conf)]
async fn main() {
    srand(get_time() as u64);
    let mut world = World::new();
    let mut player = Player::new(WX as f32 / 2.0, 20.0, WZ as f32 / 2.0);

    let mut faces: Vec<Face> = Vec::with_capacity(4000);
    let mut active_touches: [Option<(u64, bool, bool, i32, Vec2)>; 10] = Default::default();

    let mut render_dist = 22.0;
    let mut fps_timer = 0.0;
    let mut fps_display = 0;

    loop {
        let dt = get_frame_time().min(0.05);
        let sw = screen_width();
        let sh = screen_height();
        let scale = sh / 800.0;

        let mut move_dir = Vec2::ZERO;
        let mut jump = false;
        let mut break_block = false;
        let mut place_block = false;

        if is_key_down(KeyCode::W) {
            move_dir.y += 1.0;
        }
        if is_key_down(KeyCode::S) {
            move_dir.y -= 1.0;
        }
        if is_key_down(KeyCode::A) {
            move_dir.x -= 1.0;
        }
        if is_key_down(KeyCode::D) {
            move_dir.x += 1.0;
        }
        if is_key_down(KeyCode::Space) {
            jump = true;
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            break_block = true;
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            place_block = true;
        }

        if is_mouse_button_down(MouseButton::Right) {
            let md = mouse_delta_position();
            player.yaw -= md.x * 0.005;
            player.pitch = (player.pitch - md.y * 0.005).clamp(-1.48, 1.48);
        }

        for i in 1..=6 {
            let key = match i {
                1 => KeyCode::Key1,
                2 => KeyCode::Key2,
                3 => KeyCode::Key3,
                4 => KeyCode::Key4,
                5 => KeyCode::Key5,
                6 => KeyCode::Key6,
                _ => KeyCode::Unknown,
            };
            if is_key_pressed(key) {
                player.selected_block = [GRASS, DIRT, STONE, SAND, WOOD, PLANKS][i - 1];
            }
        }

        let btn_jump = Rect::new(
            sw - 120.0 * scale,
            sh - 120.0 * scale,
            80.0 * scale,
            80.0 * scale,
        );
        let btn_break = Rect::new(
            sw - 120.0 * scale,
            sh - 220.0 * scale,
            80.0 * scale,
            80.0 * scale,
        );
        let btn_place = Rect::new(
            sw - 220.0 * scale,
            sh - 120.0 * scale,
            80.0 * scale,
            80.0 * scale,
        );

        let hotbar_y = sh - 80.0 * scale;
        let hotbar_w = 60.0 * scale;
        let hotbar_start_x = sw / 2.0 - 3.0 * hotbar_w;

        let touches = touches();
        let mut current_touch_ids = vec![];

        // 1. Processar toques atuais e atualizar estado
        for touch in &touches {
            current_touch_ids.push(touch.id);
            let pos = touch.position;

            let mut found = false;
            for slot in active_touches.iter_mut() {
                if let Some((id, is_joy, is_look, _btn, prev_pos)) = slot {
                    if *id == touch.id {
                        found = true;
                        if touch.phase == TouchPhase::Moved {
                            if *is_joy {
                                let diff = pos - *prev_pos;
                                let max_r = 60.0 * scale;
                                let len = diff.length();
                                if len > 10.0 * scale {
                                    let clamped_len = len.min(max_r);
                                    move_dir = diff.normalize() * (clamped_len / max_r);
                                }
                            } else if *is_look {
                                let delta = pos - *prev_pos;
                                player.yaw -= delta.x * 0.008;
                                player.pitch = (player.pitch - delta.y * 0.008).clamp(-1.48, 1.48);
                            }
                        }
                        // Atualiza a posição anterior para o próximo frame
                        *prev_pos = pos;
                        break;
                    }
                }
            }

            // 2. Lidar com novos toques
            if !found && touch.phase == TouchPhase::Started {
                let mut btn_hit = -1;
                if btn_jump.contains(pos) {
                    btn_hit = 0;
                    jump = true;
                } else if btn_break.contains(pos) {
                    btn_hit = 1;
                    break_block = true;
                } else if btn_place.contains(pos) {
                    btn_hit = 2;
                    place_block = true;
                } else if pos.y > hotbar_y
                    && pos.x > hotbar_start_x
                    && pos.x < hotbar_start_x + 6.0 * hotbar_w
                {
                    btn_hit = 3;
                    let idx = ((pos.x - hotbar_start_x) / hotbar_w) as usize;
                    if idx < 6 {
                        player.selected_block = [GRASS, DIRT, STONE, SAND, WOOD, PLANKS][idx];
                    }
                }

                let new_touch = if btn_hit >= 0 {
                    (touch.id, false, false, btn_hit, pos)
                } else if pos.x < sw / 2.0 {
                    (touch.id, true, false, -1, pos) // Base do joystick
                } else {
                    (touch.id, false, true, -1, pos) // Base da câmera
                };

                for slot in active_touches.iter_mut() {
                    if slot.is_none() {
                        *slot = Some(new_touch);
                        break;
                    }
                }
            }
        }

        // 3. Remover toques que foram soltos
        for slot in active_touches.iter_mut() {
            if let Some((id, _, _, _, _)) = slot {
                if !current_touch_ids.contains(id) {
                    *slot = None;
                }
            }
        }

        // Fallback mobile look logic (simplified for stability)
        // for touch in touches {
        //     if touch.phase == TouchPhase::Moved && touch.position.x >= sw / 2.0 {
        //         let mut is_btn = false;
        //         if btn_jump.contains(touch.position)
        //             || btn_break.contains(touch.position)
        //             || btn_place.contains(touch.position)
        //         {
        //             is_btn = true;
        //         }
        //         if !is_btn && touch.position.y < hotbar_y {
        //             let delta = touch.position - touch.previous;
        //             player.yaw -= delta.x * 0.008;
        //             player.pitch = (player.pitch - delta.y * 0.008).clamp(-1.48, 1.48);
        //         }
        //     }
        // }

        update_physics(&mut player, &world, move_dir, jump, dt);

        let eye = player.eye_pos();
        let fwd = player.forward();
        let hit = raycast(&world, eye, fwd, 5.0);

        if break_block {
            if let Some((bpos, _)) = hit {
                world.set(bpos.x as i32, bpos.y as i32, bpos.z as i32, AIR);
            }
        }
        if place_block {
            if let Some((bpos, normal)) = hit {
                let target = bpos + normal;
                let tx = target.x as i32;
                let ty = target.y as i32;
                let tz = target.z as i32;
                let px = player.pos.x.floor() as i32;
                let py = player.pos.y.floor() as i32;
                let pz = player.pos.z.floor() as i32;
                let mut blocked = false;
                for dx in 0..1 {
                    for dy in 0..2 {
                        for dz in 0..1 {
                            if px + dx == tx && py + dy == ty && pz + dz == tz {
                                blocked = true;
                            }
                        }
                    }
                }
                if !blocked {
                    world.set(tx, ty, tz, player.selected_block);
                }
            }
        }

        clear_background(SKY_COLOR);

        for i in 0..24 {
            let t = i as f32 / 24.0;
            let c = lerp_color(Color::new(0.6, 0.8, 1.0, 1.0), SKY_COLOR, t);
            draw_rectangle(0.0, sh * t, sw, sh / 24.0 + 1.0, c);
        }

        let sun_x = sw * 0.8;
        let sun_y = sh * 0.2;
        let sun_r = 40.0 * scale;
        for i in 0..16 {
            let a1 = i as f32 * std::f32::consts::PI * 2.0 / 16.0;
            let a2 = (i + 1) as f32 * std::f32::consts::PI * 2.0 / 16.0;
            let p1 = Vec2::new(sun_x + a1.cos() * sun_r, sun_y + a1.sin() * sun_r);
            let p2 = Vec2::new(sun_x + a2.cos() * sun_r, sun_y + a2.sin() * sun_r);
            draw_triangle(
                Vec2::new(sun_x, sun_y),
                p1,
                p2,
                Color::new(1.0, 0.9, 0.5, 1.0),
            );
        }

        let time = get_time() as f32;
        for i in 0..5 {
            let cx = ((time * 5.0 + i as f32 * 300.0) % (sw + 200.0)) - 100.0;
            let cy = 50.0 * scale + i as f32 * 30.0 * scale;
            draw_rectangle(
                cx,
                cy,
                120.0 * scale,
                30.0 * scale,
                Color::new(1.0, 1.0, 1.0, 0.8),
            );
            draw_rectangle(
                cx + 30.0 * scale,
                cy - 20.0 * scale,
                80.0 * scale,
                30.0 * scale,
                Color::new(1.0, 1.0, 1.0, 0.8),
            );
        }

        faces.clear();
        let max_dist_sq = render_dist * render_dist;
        let px = player.pos.x as i32;
        let py = player.pos.y as i32;
        let pz = player.pos.z as i32;
        let rd = render_dist as i32;
        let cam_fwd = player.forward();

        for x in (px - rd).max(0)..(px + rd).min(WX as i32 - 1) {
            for z in (pz - rd).max(0)..(pz + rd).min(WZ as i32 - 1) {
                for y in (py - rd).max(0)..(py + rd).min(WY as i32 - 1) {
                    let b = world.get(x, y, z);
                    if b == AIR {
                        continue;
                    }

                    let dx = x as f32 + 0.5 - player.pos.x;
                    let dy = y as f32 + 0.5 - (player.pos.y + 1.6);
                    let dz = z as f32 + 0.5 - player.pos.z;
                    let dist_sq = dx * dx + dy * dy + dz * dz;

                    if dist_sq > max_dist_sq {
                        continue;
                    }

                    let len = dist_sq.sqrt();
                    if len > 0.1 {
                        let dot = (dx * cam_fwd.x + dy * cam_fwd.y + dz * cam_fwd.z) / len;
                        if dot < 0.3 && dist_sq > 20.0 {
                            continue;
                        }
                    }

                    let _is_water = b == WATER;
                    let lod = dist_sq < 36.0;

                    let neighbors = [
                        world.get(x, y + 1, z),
                        world.get(x, y - 1, z),
                        world.get(x, y, z + 1),
                        world.get(x, y, z - 1),
                        world.get(x + 1, y, z),
                        world.get(x - 1, y, z),
                    ];

                    for f in 0..6 {
                        let n = neighbors[f];
                        let should_draw = n == AIR || (b != WATER && n == WATER);
                        if !should_draw {
                            continue;
                        }

                        let verts = match f {
                            0 => [
                                Vec3::new(x as f32, (y + 1) as f32, z as f32),
                                Vec3::new((x + 1) as f32, (y + 1) as f32, z as f32),
                                Vec3::new((x + 1) as f32, (y + 1) as f32, (z + 1) as f32),
                                Vec3::new(x as f32, (y + 1) as f32, (z + 1) as f32),
                            ],
                            1 => [
                                Vec3::new(x as f32, y as f32, (z + 1) as f32),
                                Vec3::new((x + 1) as f32, y as f32, (z + 1) as f32),
                                Vec3::new((x + 1) as f32, y as f32, z as f32),
                                Vec3::new(x as f32, y as f32, z as f32),
                            ],
                            2 => [
                                Vec3::new(x as f32, y as f32, (z + 1) as f32),
                                Vec3::new(x as f32, (y + 1) as f32, (z + 1) as f32),
                                Vec3::new((x + 1) as f32, (y + 1) as f32, (z + 1) as f32),
                                Vec3::new((x + 1) as f32, y as f32, (z + 1) as f32),
                            ],
                            3 => [
                                Vec3::new((x + 1) as f32, y as f32, z as f32),
                                Vec3::new((x + 1) as f32, (y + 1) as f32, z as f32),
                                Vec3::new(x as f32, (y + 1) as f32, z as f32),
                                Vec3::new(x as f32, y as f32, z as f32),
                            ],
                            4 => [
                                Vec3::new((x + 1) as f32, y as f32, (z + 1) as f32),
                                Vec3::new((x + 1) as f32, (y + 1) as f32, (z + 1) as f32),
                                Vec3::new((x + 1) as f32, (y + 1) as f32, z as f32),
                                Vec3::new((x + 1) as f32, y as f32, z as f32),
                            ],
                            5 => [
                                Vec3::new(x as f32, y as f32, z as f32),
                                Vec3::new(x as f32, (y + 1) as f32, z as f32),
                                Vec3::new(x as f32, (y + 1) as f32, (z + 1) as f32),
                                Vec3::new(x as f32, y as f32, (z + 1) as f32),
                            ],
                            _ => continue,
                        };

                        let mut screen_verts = [Vec2::ZERO; 4];
                        let mut valid = true;
                        for i in 0..4 {
                            if let Some(sv) = project(verts[i], &player, sw, sh) {
                                screen_verts[i] = sv;
                            } else {
                                valid = false;
                                break;
                            }
                        }
                        if !valid {
                            continue;
                        }

                        let cross = (screen_verts[1].x - screen_verts[0].x)
                            * (screen_verts[2].y - screen_verts[0].y)
                            - (screen_verts[1].y - screen_verts[0].y)
                                * (screen_verts[2].x - screen_verts[0].x);
                        if cross < 0.0 {
                            continue;
                        }

                        let mut color = block_color(b, f, x, y, z);
                        let fog_t = (dist_sq / max_dist_sq).clamp(0.0, 1.0).powf(1.5);
                        color = lerp_color(color, FOG_COLOR, fog_t);

                        if faces.len() < 3500 {
                            faces.push(Face {
                                v: screen_verts,
                                color,
                                dist_sq,
                                lod,
                            });
                        }
                    }
                }
            }
        }

        faces.sort_unstable_by(|a, b| {
            b.dist_sq
                .partial_cmp(&a.dist_sq)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for face in &faces {
            draw_face(face);
        }

        if let Some((bpos, _)) = hit {
            let verts = [
                Vec3::new(bpos.x, bpos.y, bpos.z),
                Vec3::new(bpos.x + 1.0, bpos.y, bpos.z),
                Vec3::new(bpos.x + 1.0, bpos.y + 1.0, bpos.z),
                Vec3::new(bpos.x, bpos.y + 1.0, bpos.z),
                Vec3::new(bpos.x, bpos.y, bpos.z + 1.0),
                Vec3::new(bpos.x + 1.0, bpos.y, bpos.z + 1.0),
                Vec3::new(bpos.x + 1.0, bpos.y + 1.0, bpos.z + 1.0),
                Vec3::new(bpos.x, bpos.y + 1.0, bpos.z + 1.0),
            ];
            let edges = [
                (0, 1),
                (1, 2),
                (2, 3),
                (3, 0),
                (4, 5),
                (5, 6),
                (6, 7),
                (7, 4),
                (0, 4),
                (1, 5),
                (2, 6),
                (3, 7),
            ];
            for (a, b) in edges {
                if let Some(p1) = project(verts[a], &player, sw, sh) {
                    if let Some(p2) = project(verts[b], &player, sw, sh) {
                        let dx = p2.x - p1.x;
                        let dy = p2.y - p1.y;
                        let len = (dx * dx + dy * dy).sqrt();
                        if len > 0.1 {
                            let nx = -dy / len * 1.5;
                            let ny = dx / len * 1.5;
                            draw_triangle(
                                p1 + Vec2::new(nx, ny),
                                p1 - Vec2::new(nx, ny),
                                p2 + Vec2::new(nx, ny),
                                Color::new(0.0, 0.0, 0.0, 0.8),
                            );
                            draw_triangle(
                                p1 - Vec2::new(nx, ny),
                                p2 - Vec2::new(nx, ny),
                                p2 + Vec2::new(nx, ny),
                                Color::new(0.0, 0.0, 0.0, 0.8),
                            );
                        }
                    }
                }
            }
        }

        let cx = sw / 2.0;
        let cy = sh / 2.0;
        draw_rectangle(cx - 10.0, cy - 1.0, 20.0, 2.0, WHITE);
        draw_rectangle(cx - 1.0, cy - 10.0, 2.0, 20.0, WHITE);

        for touch in &touches {
            if touch.position.x < sw / 2.0 && touch.phase == TouchPhase::Stationary
                || touch.phase == TouchPhase::Moved
            {
                // Simple visual joystick feedback
                draw_circle(
                    touch.position.x,
                    touch.position.y,
                    60.0 * scale,
                    Color::new(1.0, 1.0, 1.0, 0.2),
                );
            }
        }

        draw_rectangle(
            btn_jump.x,
            btn_jump.y,
            btn_jump.w,
            btn_jump.h,
            Color::new(0.2, 0.8, 0.2, 0.5),
        );
        draw_text(
            "JUMP",
            btn_jump.x + 10.0 * scale,
            btn_jump.y + 45.0 * scale,
            20.0 * scale,
            WHITE,
        );

        draw_rectangle(
            btn_break.x,
            btn_break.y,
            btn_break.w,
            btn_break.h,
            Color::new(0.8, 0.2, 0.2, 0.5),
        );
        draw_text(
            "BREAK",
            btn_break.x + 5.0 * scale,
            btn_break.y + 45.0 * scale,
            20.0 * scale,
            WHITE,
        );

        draw_rectangle(
            btn_place.x,
            btn_place.y,
            btn_place.w,
            btn_place.h,
            Color::new(0.2, 0.2, 0.8, 0.5),
        );
        draw_text(
            "PLACE",
            btn_place.x + 5.0 * scale,
            btn_place.y + 45.0 * scale,
            20.0 * scale,
            WHITE,
        );

        let blocks = [GRASS, DIRT, STONE, SAND, WOOD, PLANKS];
        for i in 0..6 {
            let rx = hotbar_start_x + i as f32 * hotbar_w;
            let col = block_color(blocks[i], 0, 0, 0, 0);
            draw_rectangle(rx, hotbar_y, hotbar_w - 2.0, hotbar_w - 2.0, col);
            if player.selected_block == blocks[i] {
                draw_rectangle(rx - 2.0, hotbar_y - 2.0, hotbar_w + 2.0, 2.0, WHITE);
                draw_rectangle(rx - 2.0, hotbar_y + hotbar_w, hotbar_w + 2.0, 2.0, WHITE);
                draw_rectangle(rx - 2.0, hotbar_y, 2.0, hotbar_w, WHITE);
                draw_rectangle(rx + hotbar_w, hotbar_y, 2.0, hotbar_w, WHITE);
            }
        }

        fps_timer += dt;
        if fps_timer > 0.5 {
            fps_display = (1.0 / dt) as i32;
            fps_timer = 0.0;

            if dt > 0.040 && render_dist > 12.0 {
                render_dist -= 1.0;
            } else if dt < 0.025 && render_dist < 24.0 {
                render_dist += 1.0;
            }
        }
        draw_text(
            &format!("FPS: {} | Dist: {}", fps_display, render_dist as i32),
            10.0,
            30.0,
            20.0,
            WHITE,
        );

        next_frame().await;
    }
}
