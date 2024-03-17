pub mod density;
pub mod obstacle;

use self::{density::DensityField, obstacle::ObstacleField};
use super::layout::FieldLayout;
use crate::prelude::*;

#[derive(Resource)]
pub struct CostFields {
    pub density: DensityField,
    pub obstacle: ObstacleField,
}

impl CostFields {
    pub fn from_layout(layout: &FieldLayout) -> Self {
        Self { density: DensityField::from_layout(layout), obstacle: ObstacleField::from_layout(layout) }
    }
}
