use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct ButtonLayout {
    pub tap: Vec<Vec2>,
    pub a: Vec<Vec2>,
    pub b: Vec<Vec2>,
    pub c: Vec<Vec2>,
    pub d: Vec<Vec2>,
    pub e: Vec<Vec2>,
    pub tap_spawn: Vec<Vec2>,
}

impl Default for ButtonLayout {
    fn default() -> Self {
        let mut tap = Vec::new();
        let mut a = Vec::new();
        let mut b = Vec::new();
        let mut c = Vec::new();
        let mut d = Vec::new();
        let mut e = Vec::new();
        let mut tap_spawn = Vec::new();
        for i in 0..8 {
            let a1 = std::f32::consts::FRAC_PI_8 + i as f32 * std::f32::consts::FRAC_PI_4;
            let a2 = i as f32 * std::f32::consts::FRAC_PI_4;
            tap.push(Vec2::new(1.0 * a1.cos(), 1.0 * a1.sin()));
            a.push(Vec2::new(4.1 * 4.8 * a1.cos(), 4.1 / 4.8 * a1.sin()));
            d.push(Vec2::new(4.1 * 4.8 * a2.cos(), 4.1 / 4.8 * a2.sin()));
            b.push(Vec2::new(2.3 * 4.8 * a1.cos(), 2.3 / 4.8 * a1.sin()));
            e.push(Vec2::new(3.0 * 4.8 * a2.cos(), 3.0 / 4.8 * a2.sin()));
            tap_spawn.push(Vec2::new(1.225 * a1.cos(), 1.225 * a1.sin()));
        }
        for _ in 0..3 {
            c.push(Vec2::ZERO);
        }

        Self {
            tap,
            a,
            b,
            c,
            d,
            e,
            tap_spawn,
        }
    }
}
