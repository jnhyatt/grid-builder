use std::collections::{BTreeMap, HashMap};

use bevy::{
    ecs::system::Resource,
    math::{Ray2d, Vec2, Vec3},
};
use is_odd::IsOdd;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Resource, Debug, Default, Clone)]
pub struct Board {
    pub cells: Vec<Cell>,
    pub meshes: Vec<BoardMesh>,
}

impl Board {
    pub fn pick(&self, pos: Vec2) -> Option<usize> {
        self.cells.iter().position(|x| x.shape.contains(pos))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Cell {
    pub neighbors: HashMap<usize, Path>,
    pub shape: Polygon,
    pub position: Vec2,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BoardMesh {
    pub color: BoardColor,
    pub mesh: Mesh,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Path(pub BTreeMap<Keyframe, Vec2>);

impl Path {
    pub fn simple(start: Vec2, end: Vec2) -> Self {
        Self([(Keyframe(0.0), start), (Keyframe(1.0), end)].into())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Polygon {
    pub points: Vec<Vec2>,
}

impl Polygon {
    fn line_segments(&self) -> impl Iterator<Item = LineSegment> + '_ {
        self.points
            .iter()
            .copied()
            .circular_tuple_windows()
            .map(|(x, y)| LineSegment(x, y))
    }

    pub fn contains(&self, pos: Vec2) -> bool {
        self.line_segments()
            .filter(|x| x.intersection(Ray2d::new(pos, Vec2::X)).is_some())
            .count()
            .is_odd()
    }
}

impl std::ops::Add<Vec2> for Polygon {
    type Output = Polygon;

    fn add(self, rhs: Vec2) -> Self::Output {
        Self {
            points: self.points.into_iter().map(|x| x + rhs).collect(),
        }
    }
}

#[derive(Debug)]
struct LineSegment(Vec2, Vec2);

impl LineSegment {
    fn ab(&self) -> Vec2 {
        self.1 - self.0
    }

    fn lerp(&self, t: f32) -> Vec2 {
        self.0 + self.ab() * t
    }

    fn intersection(&self, ray: Ray2d) -> Option<Vec2> {
        let t = (ray.origin - self.0).perp_dot(*ray.direction) / self.ab().perp_dot(*ray.direction);
        let u = (self.0 - ray.origin).perp_dot(self.ab()) / ray.direction.perp_dot(self.ab());
        (u >= 0.0 && (0.0..=1.0).contains(&t)).then(|| self.lerp(t))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BoardColor {
    PlayerColor,
    StaticColor(f32, f32, f32),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Mesh {
    IndexedLineMesh {
        vertices: Vec<Vec3>,
        lines: Vec<[usize; 2]>,
    },
    IndexedTriMesh {
        vertices: Vec<Vec3>,
        triangles: Vec<[usize; 3]>,
    },
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, PartialOrd, Debug)]
pub struct Keyframe(pub f32);

impl Eq for Keyframe {}

impl Ord for Keyframe {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}
