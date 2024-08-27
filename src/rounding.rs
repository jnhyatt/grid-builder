pub trait Rounding {
    fn round_to_int(self) -> i32;
    fn round_with_diff(self) -> (i32, f32);
}

impl Rounding for f32 {
    fn round_to_int(self) -> i32 {
        self.round() as i32
    }

    fn round_with_diff(self) -> (i32, f32) {
        let it = self.round_to_int();
        (it, (self - it as f32).abs())
    }
}
