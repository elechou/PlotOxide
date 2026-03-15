pub mod axis;
pub mod data;
mod geometry;
pub mod grid_removal;
pub mod mask;
mod pixels;
mod spatial;

pub use self::pixels::detect_background_color;
