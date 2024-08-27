use bevy::{gizmos::gizmos::Gizmos, math::Vec2, render::color::Color};

pub trait CustomGizmos {
    fn square(&mut self, pos: Vec2, color: Color);
    fn hex(&mut self, pos: Vec2, color: Color);
}

impl CustomGizmos for Gizmos<'_, '_> {
    fn square(&mut self, pos: Vec2, color: Color) {
        self.linestrip_2d(
            [
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(-1.0, -1.0),
            ]
            .map(|x| x + pos),
            color,
        )
    }

    fn hex(&mut self, pos: Vec2, color: Color) {
        self.linestrip_2d(
            [
                Vec2::new(1.0, 0.0),
                Vec2::new(0.5, 0.866),
                Vec2::new(-0.5, 0.866),
                Vec2::new(-1.0, 0.0),
                Vec2::new(-0.5, -0.866),
                Vec2::new(0.5, -0.866),
                Vec2::new(1.0, 0.0),
            ]
            .map(|x| x + pos),
            color,
        )
    }
}
