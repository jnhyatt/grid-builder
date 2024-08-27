use std::hash::Hash;

use bevy::math::Vec2;

use crate::board::Polygon;

pub trait BaseCell: std::fmt::Debug + Copy + Eq + Hash + Ord {
    type Corner: BaseCorner;

    fn pick(pos: Vec2) -> Self;
    fn position(&self) -> Vec2;
    fn neighbors(&self) -> Vec<Self>
    where
        Self: Sized;
    fn shape(&self) -> Polygon;
    fn corners(&self) -> Vec<Self::Corner>;
    fn lines(&self) -> Vec<Edge<Self>>;

    fn adjacent_to(&self, other: &Self) -> bool
    where
        Self: Sized,
    {
        self.neighbors().into_iter().any(|x| x == *other)
    }

    fn neighboring_edge(&self, other: &Self) -> Edge<Self> {
        let my_edges = self.lines();
        let mut neighbor_edges = other.lines();
        for e in &mut neighbor_edges {
            e.reverse();
        }
        my_edges
            .into_iter()
            .filter(|x| neighbor_edges.contains(x))
            .next()
            .expect("Edges are not adjacent!")
    }
}

pub type Edge<C> = [<C as BaseCell>::Corner; 2];

pub trait BaseCorner: std::fmt::Debug + Copy + Eq + Hash + Ord {
    fn position(&self) -> Vec2;
}

pub mod square {
    use bevy::math::Vec2;

    use crate::{board::Polygon, rounding::Rounding};

    use super::{BaseCell, BaseCorner};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
    pub struct Cell {
        pub x: i32,
        pub y: i32,
    }

    impl BaseCell for Cell {
        type Corner = Corner;

        fn pick(pos: Vec2) -> Self {
            Self {
                x: (pos.x / 2.0).round_to_int(),
                y: (pos.y / 2.0).round_to_int(),
            }
        }

        fn position(&self) -> Vec2 {
            Vec2::new(self.x as f32, self.y as f32) * 2.0
        }

        fn neighbors(&self) -> Vec<Self> {
            [
                Self { x: 1, y: 0 },
                Self { x: 0, y: 1 },
                Self { x: -1, y: 0 },
                Self { x: 0, y: -1 },
            ]
            .map(|x| Self {
                x: x.x + self.x,
                y: x.y + self.y,
            })
            .into()
        }

        fn shape(&self) -> Polygon {
            Polygon {
                points: [
                    Vec2::new(-1.0, -1.0),
                    Vec2::new(1.0, -1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(-1.0, 1.0),
                ]
                .iter()
                .map(|&x| x + self.position())
                .collect(),
            }
        }

        fn corners(&self) -> Vec<Corner> {
            [
                Corner {
                    x: self.x,
                    y: self.y,
                },
                Corner {
                    x: self.x + 1,
                    y: self.y,
                },
                Corner {
                    x: self.x + 1,
                    y: self.y + 1,
                },
                Corner {
                    x: self.x,
                    y: self.y + 1,
                },
            ]
            .into()
        }

        fn lines(&self) -> Vec<[Self::Corner; 2]> {
            let corners = self.corners();
            [
                [corners[0], corners[1]],
                [corners[1], corners[2]],
                [corners[2], corners[3]],
                [corners[3], corners[0]],
            ]
            .into()
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
    pub struct Corner {
        pub x: i32,
        pub y: i32,
    }

    impl BaseCorner for Corner {
        fn position(&self) -> Vec2 {
            Vec2::new(self.x as f32, self.y as f32) * 2.0 - Vec2::ONE
        }
    }
}

pub mod hex {
    use bevy::math::Vec2;

    use crate::{board::Polygon, rounding::Rounding};

    use super::{BaseCell, BaseCorner};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
    pub struct Cell {
        pub q: i32,
        pub r: i32,
    }

    impl BaseCell for Cell {
        type Corner = Corner;

        fn corners(&self) -> Vec<Self::Corner> {
            [
                Corner {
                    q: self.q,
                    r: self.r,
                    left: false,
                },
                Corner {
                    q: self.q + 1,
                    r: self.r,
                    left: true,
                },
                Corner {
                    q: self.q - 1,
                    r: self.r + 1,
                    left: false,
                },
                Corner {
                    q: self.q,
                    r: self.r,
                    left: true,
                },
                Corner {
                    q: self.q - 1,
                    r: self.r,
                    left: false,
                },
                Corner {
                    q: self.q + 1,
                    r: self.r - 1,
                    left: true,
                },
            ]
            .into()
        }

        fn lines(&self) -> Vec<[Self::Corner; 2]> {
            let corners = self.corners();
            [
                [corners[0], corners[1]],
                [corners[1], corners[2]],
                [corners[2], corners[3]],
                [corners[3], corners[4]],
                [corners[4], corners[5]],
                [corners[5], corners[0]],
            ]
            .into()
        }

        fn neighbors(&self) -> Vec<Self> {
            [
                Self {
                    q: self.q - 1,
                    r: self.r,
                },
                Self {
                    q: self.q - 1,
                    r: self.r + 1,
                },
                Self {
                    q: self.q,
                    r: self.r - 1,
                },
                Self {
                    q: self.q,
                    r: self.r + 1,
                },
                Self {
                    q: self.q + 1,
                    r: self.r - 1,
                },
                Self {
                    q: self.q + 1,
                    r: self.r,
                },
            ]
            .into()
        }

        fn pick(pos: Vec2) -> Self {
            let cq = 2.0 * pos.x / 3.0;
            let cr = -1.0 / 3.0 * pos.x + 3.0f32.sqrt() / 3.0 * pos.y;
            let (q, dq) = cq.round_with_diff();
            let (r, dr) = cr.round_with_diff();
            let (s, ds) = (-cq - cr).round_with_diff();
            if dq > dr && dq > ds {
                Self { q: -r - s, r }
            } else if dr > ds {
                Self { q, r: -q - s }
            } else {
                Self { q, r }
            }
        }

        fn position(&self) -> Vec2 {
            Vec2::new(
                self.q as f32 * 3.0 / 2.0,
                3.0f32.sqrt() / 2.0 * self.q as f32 + 3.0f32.sqrt() * self.r as f32,
            )
        }

        fn shape(&self) -> Polygon {
            Polygon {
                points: [
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.5, 0.866),
                    Vec2::new(-0.5, 0.866),
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(-0.5, -0.866),
                    Vec2::new(0.5, -0.866),
                ]
                .iter()
                .map(|&x| x + self.position())
                .collect(),
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
    pub struct Corner {
        pub q: i32,
        pub r: i32,
        pub left: bool,
    }

    impl BaseCorner for Corner {
        fn position(&self) -> Vec2 {
            let &Corner { q, r, left } = self;
            Cell { q, r }.position() + if left { Vec2::NEG_X } else { Vec2::X }
        }
    }
}
